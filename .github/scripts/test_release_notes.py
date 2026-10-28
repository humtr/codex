import tempfile
from pathlib import Path
import unittest
import release_notes


class ReleaseNotes(unittest.TestCase):
    def test_publication_contains_only_the_packaged_component_notes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root/'release').mkdir()
            notes = root/'release/notes.md'
            notes.write_text('## Core\n- Core update.\n\n## Manager\n- Manager update.\n\n## TUI\n- TUI update.\n\n## Provider\n- Provider update.\n')
            self.assertEqual(release_notes.render(root, 'provider'), '## Provider\n- Provider update.\n')
            self.assertEqual(release_notes.render(root, 'base'), '## Core\n- Core update.\n')
            self.assertEqual(release_notes.render(root, 'full'), '## Core\n- Core update.\n\n## Manager\n- Manager update.\n\n## TUI\n- TUI update.\n')
            for invalid, component in [
                ('## Core\n- Only Core.\n', 'full'),
                ('## Provider\n- One.\n## Provider\n- Duplicate.\n', 'provider'),
                ('## Provider\n- Bad\x1b[2J.\n', 'provider'),
            ]:
                notes.write_text(invalid)
                with self.assertRaises(ValueError):
                    release_notes.render(root, component)
