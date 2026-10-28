"""
Tool to search and download manga chapters from Pixiv API (https://image.chiraitori.io.vn)
Usage:
  python scripts/fetch_pixiv_manga.py --id 151373838 --output showcase_samples/kaguya_4koma
  python scripts/fetch_pixiv_manga.py --search "4コマ" --page 1
"""

import argparse
import json
import os
import sys
import urllib.parse
import urllib.request

sys.stdout.reconfigure(encoding='utf-8')

API_BASE = "https://image.chiraitori.io.vn"

def search_manga(keyword="4コマ", page=1):
    q = urllib.parse.quote(keyword)
    url = f"{API_BASE}/api/search?keyword={q}&page={page}"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    
    illusts = (data.get("data") or {}).get("illusts") or []
    multipage = [it for it in illusts if it.get("pageCount", 1) > 1]
    print(f"\n--- Search results for '{keyword}' (Page {page}) ---")
    print(f"Total multi-page works found: {len(multipage)}")
    for m in multipage:
        print(f"ID: {m['id']} | Title: {m['title']} | Pages: {m['pageCount']} | Author: {m.get('userName')}")
    return multipage

def download_work(illust_id, output_dir=None):
    url = f"{API_BASE}/api/illust/{illust_id}?pages=true"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    
    pages = data.get("data")
    if not pages or not isinstance(pages, list):
        print(f"No pages found for illust {illust_id}")
        return []

    if not output_dir:
        output_dir = os.path.join("showcase_samples", f"illust_{illust_id}")
    os.makedirs(output_dir, exist_ok=True)

    downloaded = []
    print(f"\nDownloading {len(pages)} pages for illust {illust_id} into {output_dir}...")
    for idx, p in enumerate(pages):
        page_url = p.get("regular") or p.get("original")
        if not page_url:
            continue
        proxy_url = f"{API_BASE}/api/image/?url={urllib.parse.quote(page_url)}"
        r = urllib.request.Request(proxy_url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(r) as res:
            content = res.read()
        
        ext = ".png" if ".png" in page_url else ".jpg"
        target_path = os.path.join(output_dir, f"page_{idx+1:02d}{ext}")
        with open(target_path, "wb") as f:
            f.write(content)
        print(f"  ✓ Saved page {idx+1}/{len(pages)}: {target_path} ({len(content):,} bytes)")
        downloaded.append(target_path)

    print(f"Completed! Downloaded {len(downloaded)} pages to {output_dir}")
    return downloaded

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Fetch manga from Pixiv API")
    parser.add_argument("--id", type=int, help="Pixiv illustration ID")
    parser.add_argument("--search", type=str, help="Search keyword")
    parser.add_argument("--page", type=int, default=1, help="Search page number")
    parser.add_argument("--output", type=str, help="Output directory")
    args = parser.parse_args()

    if args.id:
        download_work(args.id, args.output)
    elif args.search:
        search_manga(args.search, args.page)
    else:
        # Default: search 4-koma
        search_manga("4コマ", 1)
