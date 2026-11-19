"""Small adversarial instruction fixtures, independent of firmware implementation."""
import unittest
import xtensa_frames as a
class Words:
 def __init__(self,words):self.words=words
 def word(self,at):return self.words[at]
class Frames(unittest.TestCase):
 def parse(self,code,words=None):return a.analyze_function(Words(words or {}),{'name':'fixture','address':0x100,'size':100},a.instructions(code))
 def test_entry_literal_sub_movsp_sums_both_allocations(self):
  r=self.parse('100: 004136 entry a1, 32\n103: 000081 l32r a8, 200 <literal>\n106: c08180 sub a8, a1, a8\n109: 001810 movsp a1, a8',{0x200:231024})
  self.assertEqual((r['entry_bytes'],r['additional_bytes'],r['frame_bytes']),(32,231024,231056));self.assertFalse(r['unresolved_sp'])
 def test_negative_literal_add_and_move_are_decoded(self):
  r=self.parse('100: 004136 entry a1, 32\n103: 000081 l32r a8, 200\n106: 808180 add a8, a1, a8\n109: 001810 movsp a1, a8',{0x200:0xffff0000})
  self.assertEqual(r['frame_bytes'],65568)
 def test_movi_neg_add_alternative_is_decoded(self):
  r=self.parse('100: 004136 entry a1, 32\n103: 000082 movi a8, 1024\n106: 000080 neg a8, a8\n109: 808180 add a8, a1, a8\n10c: 001810 movsp a1, a8')
  self.assertEqual(r['frame_bytes'],1056)
 def test_unknown_input_or_clobber_cannot_pass_sp_audit(self):
  for load in ['l32i a8, a2, 0','l8ui a8, a2, 0']:
   r=self.parse(f'100: 004136 entry a1, 32\n103: 000081 {load}\n106: c08180 sub a8, a1, a8\n109: 001810 movsp a1, a8')
   self.assertTrue(r['unresolved_sp'])
 def test_body_stack_change_after_call_is_rejected(self):
  r=self.parse('100: 004136 entry a1, 32\n103: 000005 call0 400\n106: 000082 movi a8, 1024\n109: c08180 sub a8, a1, a8\n10c: 001810 movsp a1, a8')
  self.assertTrue(any('outside' in x['reason']for x in r['unresolved_sp']))
if __name__=='__main__':unittest.main()
