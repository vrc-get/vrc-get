import json, urllib.request, time
cves = ["CVE-2026-102764", "CVE-2026-104935", "CVE-2026-0322", "CVE-2026-21589", "CVE-2026-76519"]
for cve in cves:
    print("===", cve)
    try:
        with urllib.request.urlopen("https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=" + cve, timeout=40) as r:
            d = json.load(r)
    except Exception as e:
        print("  error:", e); time.sleep(10); continue
    for v in d.get("vulnerabilities", []):
        c = v["cve"]
        m = c.get("metrics", {})
        print("  status:", c.get("vulnStatus"), "| published:", c.get("published"))
        for k in ("cvssMetricV40", "cvssMetricV31", "cvssMetricV30"):
            for mm in m.get(k, []):
                cd = mm["cvssData"]
                print("  {}: {} {} ({}) src={}".format(k, cd.get("baseScore"), cd.get("baseSeverity"), cd.get("vectorString"), mm.get("source")))
    time.sleep(9)
