import json, sys, urllib.request, time

cvEs = [
    "CVE-2026-109618", "CVE-2026-109619", "CVE-2026-74580", "CVE-2026-105740",
    "CVE-2026-0322", "CVE-2026-102764", "CVE-2026-104935", "CVE-2026-76519",
    "CVE-2026-21589", "CVE-2026-96951", "CVE-2026-96656", "CVE-2026-106931",
]

for cve in cvEs:
    print("===", cve)
    url = "https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=" + cve
    try:
        with urllib.request.urlopen(url, timeout=40) as r:
            d = json.load(r)
    except Exception as e:
        print("  error:", e)
        time.sleep(1)
        continue
    for v in d.get("vulnerabilities", []):
        c = v["cve"]
        desc = [x["value"] for x in c["descriptions"] if x["lang"] == "en"]
        print("  status:", c.get("vulnStatus"), "| published:", c.get("published"))
        m = c.get("metrics", {})
        for k in ("cvssMetricV40", "cvssMetricV31", "cvssMetricV30"):
            for mm in m.get(k, []):
                cd = mm["cvssData"]
                print("  {}: {} {} ({}) src={}".format(
                    k, cd.get("baseScore"), cd.get("baseSeverity"),
                    cd.get("vectorString"), mm.get("source")))
        print("  desc:", (desc[0][:260] if desc else ""))
    time.sleep(0.8)
