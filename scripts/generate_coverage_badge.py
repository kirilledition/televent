import sys
import xml.etree.ElementTree as ET
import json
import os

def generate_badge(report_path="cobertura.xml", output_path="coverage.json"):
    try:
        tree = ET.parse(report_path)
        root = tree.getroot()
        line_rate = float(root.attrib.get("line-rate", 0))
        coverage = line_rate * 100

        color = "red"
        if coverage >= 80:
            color = "brightgreen"
        elif coverage >= 50:
            color = "yellow"

        badge = {
            "schemaVersion": 1,
            "label": "coverage",
            "message": f"{coverage:.1f}%",
            "color": color
        }

        with open(output_path, "w") as f:
            json.dump(badge, f, indent=2)

        print(f"Generated badge: {json.dumps(badge)}")

    except Exception as e:
        print(f"Error generating badge: {e}")
        sys.exit(1)

if __name__ == "__main__":
    report_file = sys.argv[1] if len(sys.argv) > 1 else "cobertura.xml"
    output_file = sys.argv[2] if len(sys.argv) > 2 else "coverage.json"
    generate_badge(report_file, output_file)
