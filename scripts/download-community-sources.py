"""Download public recognition sources; never install software or run bucket hooks.

python scripts/download-community-sources.py --out PRIVATE_SCRATCH_DIRECTORY
python scripts/build-community-catalog.py --sources PRIVATE_SCRATCH_DIRECTORY
"""
import argparse
import hashlib
import io
import json
import pathlib
import urllib.request
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument('--out', type=pathlib.Path, required=True)
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=True)


def download(url, limit=64 * 1024 * 1024):
    req = urllib.request.Request(url, headers={'User-Agent': 'AppTide-catalog-builder'})
    with urllib.request.urlopen(req, timeout=90) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError('Public source exceeded download limit')
    return data


source = download('https://cdn.winget.microsoft.com/cache/source.msix')
(args.out / 'winget-source.msix').write_bytes(source)
with zipfile.ZipFile(io.BytesIO(source)) as archive:
    info = archive.getinfo('Public/index.db')
    if info.file_size > 512 * 1024 * 1024:
        raise ValueError('WinGet index exceeds limit')
    (args.out / 'index.db').write_bytes(archive.read(info))
for bucket in ['Main', 'Extras']:
    repo = 'ScoopInstaller/' + bucket
    revision = json.loads(download('https://api.github.com/repos/' + repo + '/commits/HEAD', 1024 * 1024))['sha']
    data = download('https://codeload.github.com/' + repo + '/zip/' + revision)
    (args.out / (bucket + '.zip')).write_bytes(data)
    entries = []
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for info in archive.infolist():
            if '/bucket/' not in info.filename or not info.filename.endswith('.json'):
                continue
            if info.file_size > 1024 * 1024:
                raise ValueError('Scoop manifest exceeds limit')
            manifest = json.loads(archive.read(info))
            entries.append({'id': pathlib.PurePosixPath(info.filename).stem,
                            'homepage': manifest.get('homepage', ''),
                            'description': manifest.get('description', ''),
                            'shortcuts': manifest.get('shortcuts', [])})
    snapshot = {'repository': repo, 'revision': revision,
                'archiveSha256': hashlib.sha256(data).hexdigest(), 'entries': entries}
    (args.out / (bucket + '.json')).write_text(json.dumps(snapshot), encoding='utf-8')
    print(f'{repo}: {len(entries)} records at {revision}')
print('Downloaded metadata only. Review source revisions before rebuilding the embedded catalogue.')
