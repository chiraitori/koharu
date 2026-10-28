import urllib.request
import json
import os
import sys

sys.stdout.reconfigure(encoding='utf-8')

MCP_URL = "http://127.0.0.1:9999/mcp"

def call_mcp(name, arguments=None, req_id=1):
    payload = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments or {}
        }
    }
    req = urllib.request.Request(
        MCP_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode("utf-8"))

def ping_mcp():
    try:
        res = call_mcp("list_projects")
        print("MCP is online! Existing projects:", res.get("result", {}))
        return True
    except Exception as e:
        print("MCP is not currently reachable:", e)
        return False

def import_showcase_project(project_name="Kaguya_Showcase", folder="showcase_samples/kaguya_4koma"):
    abs_folder = os.path.abspath(folder)
    images = [
        os.path.join(abs_folder, f)
        for f in sorted(os.listdir(abs_folder))
        if f.lower().endswith(('.jpg', '.jpeg', '.png', '.webp'))
    ]
    print(f"Found {len(images)} images in {abs_folder}")
    
    # 1. Create project
    print(f"Creating project '{project_name}'...")
    res = call_mcp("create_project", {"name": project_name}, req_id=1)
    print("Create project response:", res)

    # 2. Import images
    print("Importing images...")
    res = call_mcp("import_images", {"paths": images}, req_id=2)
    print("Import images response:", res)

    # 3. Inspect project
    print("Inspecting project...")
    res = call_mcp("inspect_project", {}, req_id=3)
    print("Project info:", res)

if __name__ == "__main__":
    if ping_mcp():
        import_showcase_project()
    else:
        print("\n=> Please launch kore-minit (or bun dev / koharu.exe) with MCP enabled on port 9999, then run this script again.")
