import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.effects import isolate


class EffectTests(unittest.TestCase):
    def test_normal_route_is_byte_for_byte_unchanged(self):
        html = '<head></head><body><script src="/assets/actual-app.js"></script></body>'
        self.assertEqual(isolate(html, '', frontend=True, tree='tree'), html)

    def test_modes_preserve_the_same_app_and_identify_the_override(self):
        for mode in ['normal', 'content-mask-off', 'header-blur-off', 'sticky-effects-off']:
            html = isolate('<head></head><script src="/assets/actual-app.js"></script>', mode,
                           frontend=True, tree='21e50dcb')
            self.assertIn('/assets/actual-app.js', html)
            self.assertIn(f'data-lab-effects="{mode}"', html)
            self.assertIn('Lab 21e50dc', html)

    def test_unknown_values_and_full_mode_are_rejected(self):
        for mode, frontend in [('arbitrary-css', True), ('content-mask-off', False)]:
            with self.assertRaises(ValueError):
                isolate('<head></head>', mode, frontend=frontend, tree='tree')

    def test_each_effect_can_be_disabled_independently(self):
        content = isolate('<head></head>', 'content-mask-off', frontend=True, tree='tree')
        header = isolate('<head></head>', 'header-blur-off', frontend=True, tree='tree')
        self.assertIn('mask-image:none', content)
        self.assertNotIn('backdrop-filter', content)
        self.assertIn('backdrop-filter:none', header)
        self.assertNotIn('mask-image', header)
