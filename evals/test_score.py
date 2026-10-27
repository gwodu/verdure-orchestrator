import unittest
from pathlib import Path

import score

GRID = '''File type = "ooTextFile"
item [1]:
    intervals [1]:
        xmin = 0
        xmax = 2.5
        text = ""
    intervals [2]:
        xmin = 2.5
        xmax = 12.4
        text = "<UNSURE>Hello how</UNSURE> um, BP was 150 over 95."
'''


class ScoreTests(unittest.TestCase):
    def test_textgrid_keeps_only_spoken_intervals_and_tags_are_stripped(self):
        utts = score.parse_textgrid(GRID)
        self.assertEqual(len(utts), 1)
        self.assertEqual(utts[0][0], 2.5)
        self.assertEqual(score.normalise(score.TAG.sub(" ", utts[0][2])), ["hello", "how", "bp", "was", "150", "over", "95"])

    def test_wer_counts_substitutions_insertions_deletions(self):
        self.assertEqual(score.wer(["a", "b", "c"], ["a", "b", "c"]), 0.0)
        self.assertAlmostEqual(score.wer(["a", "b", "c"], ["a", "x", "c"]), 1 / 3)
        self.assertAlmostEqual(score.wer(["a", "b", "c"], ["a", "c"]), 1 / 3)
        self.assertAlmostEqual(score.wer(["a", "b"], ["a", "b", "c", "d"]), 1.0)
        self.assertEqual(score.wer([], []), 0.0)

    def test_invented_numbers_are_those_in_facts_but_nowhere_in_speech(self):
        r = score.score("BP one fifty over 95, pulse 72.", "bp 150 over 95 pulse 72", [{"text": "BP 150/95", "group": "examination"}, {"text": "HbA1c 8.2 %", "group": "other"}], 60.0, 15.0)
        self.assertEqual(r["invented_numbers"], ["8.2"])
        self.assertEqual(r["numbers_kept"], 2)
        self.assertEqual(r["rtf"], 0.25)
        self.assertEqual(r["fact_groups"], ["examination", "other"])

    def test_the_fetched_reference_parses(self):
        prefix = Path(__file__).parent / "fixtures" / "primock57" / "day1_consultation01"
        if not Path(f"{prefix}_doctor.TextGrid").exists():
            self.skipTest("fixtures not fetched")
        text = score.reference_text(prefix)
        self.assertGreater(len(score.normalise(text)), 500)
        self.assertNotIn("<UNIN/>", text)


if __name__ == "__main__":
    unittest.main()
