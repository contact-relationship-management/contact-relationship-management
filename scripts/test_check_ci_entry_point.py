#!/usr/bin/env python3
"""Tests for check-ci-entry-point.py: a guard that cannot fail proves nothing (CRM-T185).

    python3 scripts/test_check_ci_entry_point.py
"""
import importlib.util
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True  # no scripts/__pycache__ in the working tree

SPEC = importlib.util.spec_from_file_location(
    "check_ci_entry_point", Path(__file__).with_name("check-ci-entry-point.py")
)
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)

STAGES = {"fmt", "test", "all", "mutants"}


def commands_bad(commands, matrix=frozenset()):
    bad, _ = check.check_commands("ci.yml", list(enumerate(commands, 1)), STAGES, set(matrix))
    return bad


class Commands(unittest.TestCase):
    def test_a_call_to_a_stage_passes(self):
        self.assertEqual(commands_bad(["scripts/ci-check.sh fmt"]), [])

    def test_a_check_run_directly_fails(self):
        self.assertIn("cargo test", commands_bad(["cargo test"])[0])

    def test_a_check_chained_onto_an_installer_fails(self):
        self.assertTrue(commands_bad(["rustup component add clippy && cargo clippy"]))
        self.assertTrue(commands_bad(["npm install --global pnpm@11 && pnpm test"]))

    def test_a_check_chained_onto_a_call_fails(self):
        self.assertTrue(commands_bad(["scripts/ci-check.sh fmt && cargo test"]))

    def test_an_unknown_stage_fails(self):
        self.assertIn("does not have", commands_bad(["scripts/ci-check.sh nope"])[0])

    def test_a_listed_installer_passes(self):
        self.assertEqual(commands_bad(["rustup component add clippy"]), [])

    def test_a_matrix_call_counts_each_stage(self):
        _, called = check.check_commands(
            "ci.yml", [(1, "scripts/ci-check.sh ${{ matrix.stage }}")], STAGES, {"fmt", "test"}
        )
        self.assertEqual(called, {"fmt", "test"})

    def test_a_matrix_of_an_unknown_stage_fails(self):
        self.assertTrue(commands_bad(["scripts/ci-check.sh ${{ matrix.stage }}"], {"nope"}))


class Parsing(unittest.TestCase):
    def test_run_reads_one_line_and_block_steps(self):
        text = "steps:\n  - run: one\n  - name: x\n    run: |\n      two\n      # note\n      three\n  - run: four\n"
        self.assertEqual([c for _, c in check.run_commands(text)], ["one", "two", "three", "four"])

    def test_woodpecker_reads_every_command(self):
        text = "steps:\n  - name: a\n    commands:\n      # note\n      - one\n      - two\n  - name: b\n    commands:\n      - three\n"
        self.assertEqual([c for _, c in check.woodpecker_commands(text)], ["one", "two", "three"])

    def test_stage_labels_and_the_all_list_are_read(self):
        script = 'ALL_STAGES=(fmt test)\ncase "${stage}" in\n  fmt)\n    ;;\n  test-db)\n    ;;\n  e2e)\n    ;;\n  *)\n    ;;\nesac\n'
        self.assertEqual(check.declared_stages(script), {"fmt", "test-db", "e2e"})
        self.assertEqual(check.all_stages(script), {"fmt", "test"})

    def test_a_multiline_all_list_is_read(self):
        self.assertEqual(check.all_stages("ALL_STAGES=(\n  fmt\n  test\n)\n"), {"fmt", "test"})


class Coverage(unittest.TestCase):
    def test_complete_coverage_passes(self):
        self.assertEqual(check.check_coverage(STAGES, {"fmt", "test", "mutants"}, {"fmt", "test"}), [])

    def test_a_stage_no_workflow_calls_fails(self):
        self.assertIn("no hosted workflow", check.check_coverage(STAGES, {"fmt", "mutants"}, {"fmt", "test"})[0])

    def test_a_stage_all_leaves_out_fails(self):
        self.assertIn("leaves out stage test", check.check_coverage(STAGES, {"fmt", "test", "mutants"}, {"fmt"})[0])

    def test_all_may_not_run_a_stage_it_excludes(self):
        self.assertTrue(check.check_coverage(STAGES, {"fmt", "test", "mutants"}, {"fmt", "test", "mutants"}))


class ThisRepository(unittest.TestCase):
    def test_the_real_files_pass(self):
        self.assertEqual(check.main(), 0)


if __name__ == "__main__":
    unittest.main()
