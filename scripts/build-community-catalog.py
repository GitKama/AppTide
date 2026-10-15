"""Build a metadata-only catalogue from downloaded public WinGet/Scoop snapshots.

Usage: python scripts/build-community-catalog.py --sources PRIVATE_DOWNLOAD_FOLDER
No Scoop commands, installer URLs, hooks or scripts are imported.
"""
import argparse, concurrent.futures, datetime, hashlib, json, pathlib, re, sqlite3, urllib.request, zipfile
from urllib.parse import quote

parser=argparse.ArgumentParser()
parser.add_argument('--sources',type=pathlib.Path,required=True)
args=parser.parse_args()
repo=pathlib.Path(__file__).resolve().parents[1]
root=args.sources
def get(url):
    return urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'AppTide-catalog-builder'}),timeout=25).read()
def norm(s): return re.sub(r'[^a-z0-9]','',s.lower())
def github(url):
    m=re.match(r'https://github\.com/([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)(?:/|$)',url)
    return m[1].removesuffix('.git') if m else ''
def scalar(text,key):
    m=re.search(r'^'+key+r':\s*([^\r\n]+)',text,re.M)
    if not m:return ''
    value=m[1].strip()
    if value.startswith('"'):
        try:return json.loads(value)
        except ValueError:return ''
    if value.startswith("'") and value.endswith("'"):return value[1:-1].replace("''", "'")
    return value if not value.startswith(('|','>','[','{','&','*','!')) else ''
c=sqlite3.connect('file:'+str((root/'index.db').resolve())+'?mode=ro',uri=True)
entries={}
for id,name in c.execute('select distinct ids.id,names.name from manifest join ids on ids.rowid=manifest.id join names on names.rowid=manifest.name'):
    e=entries.setdefault(id,{'id':id,'names':[],'publishers':[],'source':'WinGet','homepage':'','repository':''})
    if name not in e['names']:e['names'].append(name)
for id,pub in c.execute('select distinct ids.id,norm_publishers.norm_publisher from manifest join ids on ids.rowid=manifest.id join norm_publishers_map on norm_publishers_map.manifest=manifest.rowid join norm_publishers on norm_publishers.rowid=norm_publishers_map.norm_publisher'):
    if pub not in entries[id]['publishers']:entries[id]['publishers'].append(pub)
scoops=[];sources=[];github_names=set()
for bucket in ['Main','Extras']:
    snapshot=json.loads((root/(bucket+'.json')).read_text(encoding='utf-8'))
    with zipfile.ZipFile(root/(bucket+'.zip')) as archive:
        manifests={pathlib.PurePosixPath(n).stem:json.loads(archive.read(n)) for n in archive.namelist() if '/bucket/' in n and n.endswith('.json')}
    sources.append({k:snapshot[k] for k in ['repository','revision','archiveSha256']})
    for a in snapshot['entries']:
        names=[a['id']]+[s[1] for s in a['shortcuts'] if isinstance(s,list) and len(s)>1 and isinstance(s[1],str)]
        url=a['homepage'];project=github(url)
        if not project:
            # Derive repository identity from reviewed public Scoop download metadata,
            # but do not redistribute or execute URLs, scripts, hooks or commands.
            m=manifests.get(a['id'],{})
            urls=[m.get('url','')]+[v.get('url','') for v in m.get('architecture',{}).values() if isinstance(v,dict)]
            projects={github(s) for u in urls for s in (u if isinstance(u,list) else [u]) if isinstance(s,str) and github(s)}
            if len(projects)==1:project=projects.pop()
        scoops.append({'id':'scoop:'+bucket+'/'+a['id'],'names':list(dict.fromkeys(names)),'publishers':[],'source':'Scoop '+bucket,'homepage':url,'repository':project,'description':a['description'][:300]})
        if project:github_names.update(norm(n) for n in names)
seed='7zip.7zip VideoLAN.VLC Microsoft.VisualStudioCode Microsoft.PowerToys Microsoft.WindowsTerminal Microsoft.PowerShell Git.Git GitHub.GitHubDesktop Notepad++.Notepad++ ShareX.ShareX OBSProject.OBSStudio RustDesk.RustDesk LocalSend.LocalSend Obsidian.Obsidian Brave.Brave Mozilla.Firefox GIMP.GIMP Inkscape.Inkscape BlenderFoundation.Blender Audacity.Audacity HandBrake.HandBrake qBittorrent.qBittorrent OpenWhisperSystems.Signal PuTTY.PuTTY WinSCP.WinSCP Docker.DockerDesktop Python.Python.3.13 OpenJS.NodeJS.LTS Logitech.OptionsPlus Adobe.Acrobat.Reader.64-bit CPUID.CPU-Z Valve.Steam EpicGames.EpicGamesLauncher Discord.Discord Spotify.Spotify JanDeDobbeleer.OhMyPosh BurntSushi.ripgrep.MSVC 4gray.iptvnator Comfy.ComfyUI-Desktop Ollama.Ollama PrimateLabs.Geekbench.6 Blizzard.BattleNet EPSON.EpsonScan2'.split()
selected=list(dict.fromkeys(seed+sorted(id for id,e in entries.items() if any(norm(n) in github_names for n in e['names']))))[:800]
versions={}
for id,v in c.execute('select ids.id,versions.version from manifest join ids on ids.rowid=manifest.id join versions on versions.rowid=manifest.version'):
    if id in selected and re.fullmatch(r'[0-9]+(?:\.[0-9]+)*',v):
        if tuple(map(int,v.split('.')))>tuple(map(int,versions.get(id,'0').split('.'))):versions[id]=v
revision=json.loads(get('https://api.github.com/repos/microsoft/winget-pkgs/commits/HEAD'))['sha']
def enrich(id):
    if id not in versions:return None
    path='/'.join([id[0].lower(),*id.split('.'),versions[id],id+'.locale.en-US.yaml'])
    try:
        text=get('https://raw.githubusercontent.com/microsoft/winget-pkgs/'+revision+'/manifests/'+quote(path,safe='/')).decode('utf-8-sig')
        if scalar(text,'PackageIdentifier')!=id:return None
        homepage=scalar(text,'PackageUrl');project=github(homepage) or github(scalar(text,'ReleaseNotesUrl')) or github(scalar(text,'PublisherSupportUrl'))
        return id,homepage,project,scalar(text,'Publisher'),scalar(text,'ShortDescription'),path
    except Exception:return None
count=0
with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
    for result in pool.map(enrich,selected):
        if result:
            id,url,project,pub,desc,path=result;e=entries[id];e.update(homepage=url,repository=project,description=desc[:300],manifest=path)
            if pub and pub not in e['publishers']:e['publishers'].append(pub)
            count+=1
for e in entries.values():
    if not e['repository'] and e['homepage']:
        candidates={s['repository'] for s in scoops if s['repository'] and s['homepage'].rstrip('/').lower()==e['homepage'].rstrip('/').lower() and any(norm(n)==norm(sn) for n in e['names'] for sn in s['names'])}
        if len(candidates)==1:e['repository']=candidates.pop()
out={'schema':1,'generated':datetime.datetime.now(datetime.timezone.utc).date().isoformat(),'wingetSource':'https://cdn.winget.microsoft.com/cache/source.msix','wingetSha256':hashlib.sha256((root/'winget-source.msix').read_bytes()).hexdigest(),'wingetManifestRevision':revision,'scoopSources':sources,'entries':list(entries.values())+scoops}
(repo/'rust/data/community.json').write_text(json.dumps(out,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
print(json.dumps({'wingetEntries':len(entries),'scoopEntries':len(scoops),'enrichedManifests':count,'githubProjects':len({e['repository'].lower() for e in out['entries'] if e['repository']}),'bytes':(repo/'rust/data/community.json').stat().st_size}))
