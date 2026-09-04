import unittest
from pathlib import Path


EXPECTED_GROUP = (
    "group: ${{ github.workflow }}-${{ github.repository }}-"
    "${{ github.event.pull_request.number || github.run_id }}"
)
EXPECTED_CANCEL = "cancel-in-progress: ${{ github.event_name == 'pull_request' }}"


class WorkflowConcurrencyContractTest(unittest.TestCase):
    def test_pr_workflows_cancel_only_superseded_pr_runs(self) -> None:
        for workflow in Path(".github/workflows").glob("*.yml"):
            source = workflow.read_text()
            if "pull_request:" not in source:
                continue
            self.assertIn(EXPECTED_GROUP, source, workflow)
            self.assertIn(EXPECTED_CANCEL, source, workflow)

    def test_push_runs_are_limited_to_main(self) -> None:
        for workflow in Path(".github/workflows").glob("*.yml"):
            source = workflow.read_text()
            if "push:" not in source:
                continue
            self.assertIn("push:\n    branches:\n      - main", source, workflow)


if __name__ == "__main__":
    unittest.main()
