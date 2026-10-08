import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "verify_macos_sandbox", Path(__file__).parents[1] / "scripts/verify_macos_sandbox.py"
)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class SandboxVerificationTest(unittest.TestCase):
    def inspect(self, rows, states):
        with patch.object(verify, "processes", return_value=rows):
            return verify.inspect(100, lambda pid, operation, kind: states[pid])

    def test_live_required_processes_and_descendants(self):
        rows = [(101, 100, "Helper --type=gpu-process"),
                (102, 101, "Helper --type=renderer"),
                (103, 999, "Unrelated --type=renderer --no-sandbox")]
        self.assertTrue(self.inspect(rows, {101: 1, 102: 1}))

    def test_unsandboxed_renderer_fails(self):
        with self.assertRaisesRegex(RuntimeError, "not sandboxed"):
            self.inspect([(101, 100, "Helper --type=renderer")], {101: 0})

    def test_disabled_gpu_fails_even_when_seatbelt_is_present(self):
        with self.assertRaisesRegex(RuntimeError, "disabled by command line"):
            self.inspect([(101, 100, "Helper --type=gpu-process --disable-gpu-sandbox")], {101: 1})

    def test_api_errors_fail(self):
        with self.assertRaisesRegex(RuntimeError, "sandbox_check failed"):
            self.inspect([(101, 100, "Helper --type=renderer")], {101: -1})

    def test_missing_gpu_is_not_success(self):
        self.assertFalse(self.inspect([(101, 100, "Helper --type=renderer")], {101: 1}))

    def test_only_explicit_utility_exemption_is_allowed(self):
        self.assertFalse(self.inspect(
            [(101, 100, "Helper --type=utility --service-sandbox-type=none")], {101: 0}))
        with self.assertRaisesRegex(RuntimeError, "not sandboxed"):
            self.inspect([(101, 100, "Helper --type=utility --service-sandbox-type=network")], {101: 0})


if __name__ == "__main__":
    unittest.main()
