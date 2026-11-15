"""Synthetic portal metadata checks; never contact a desktop or capture pixels."""
import unittest
import wayland_capture as capture
class PortalSelection(unittest.TestCase):
    def stream(self,**properties):
        return {"streams":[(42,{"source_type":2,"size":(3840,2160),**properties})]}
    def test_only_one_bounded_consented_window_is_accepted(self):
        self.assertEqual(capture.selected_stream(self.stream(**{"pipewire-serial":1001})),(42,1001,1920,1080))
        for value in [{"streams":[]},{"streams":self.stream()["streams"]*2},self.stream(source_type=1),self.stream(size=(9000,5000)),self.stream(**{"pipewire-serial":-1})]:
            with self.assertRaises(capture.Rejected):capture.selected_stream(value)
