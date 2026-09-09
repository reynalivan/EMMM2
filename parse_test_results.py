import json
import sys

try:
    with open('test-results.json', 'r', encoding='utf-8') as f:
        data = json.load(f)

    failed_files = {}
    total_failed = 0

    for tr in data.get('testResults', []):
        file_path = tr.get('name', '').replace('\\', '/')
        file_failures = 0
        for ar in tr.get('assertionResults', []):
            if ar.get('status') == 'failed':
                file_failures += 1
                total_failed += 1

        if file_failures > 0 or tr.get('status') == 'failed':
            if file_failures == 0:
                file_failures = 1 # file failed entirely (e.g. setup error)
                total_failed += 1
            failed_files[file_path] = file_failures

    print(f"Total Failed Tests: {total_failed} across {len(failed_files)} files.\n")
    for fp, count in sorted(failed_files.items(), key=lambda x: x[1], reverse=True):
        short_name = fp.split('src/')[-1] if 'src/' in fp else fp
        print(f"- {short_name} ({count} failures)")
except Exception as e:
    print(e)
