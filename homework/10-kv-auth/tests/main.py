from collections import defaultdict
import pathlib
import pytest

SCRIPT_DIR = pathlib.Path(__file__).parent.resolve()

class PassedCounter:
    def __init__(self):
        self.passed = defaultdict(int)

    def pytest_report_teststatus(self, report, config):
        if report.when == 'call' and report.passed:
            group = report.nodeid.split('::')[1]
            self.passed[group] += 1

counter = PassedCounter()
pytest.main(['-vs', SCRIPT_DIR / 'test_server.py'], plugins=[counter])

score = 0
if counter.passed['TestRSA'] == 2:
    score += 2
if counter.passed['TestAuth'] == 10:
    score += 4
if counter.passed['TestKV'] == 14:
    score += 4

print(f"\nSCORE: {score}")