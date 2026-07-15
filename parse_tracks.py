import re

try:
    with open('embed_playlist.html', encoding='utf-16') as f:
        content = f.read()
except:
    with open('embed_playlist.html', encoding='utf-8') as f:
        content = f.read()

tracks = re.findall(r'"spotify:track:([a-zA-Z0-9]{22})"', content)
print(f"Found {len(tracks)} tracks via spotify:track")

tracks2 = re.findall(r'/track/([a-zA-Z0-9]{22})', content)
print(f"Found {len(tracks2)} tracks via /track/")

with open('embed_playlist.txt', 'w') as f:
    f.write('\n'.join(set(tracks + tracks2)))
