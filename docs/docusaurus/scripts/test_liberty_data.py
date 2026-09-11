import unittest
from liberty_data import extract_timing


def arc(kind, a='rise_constraint', b='fall_constraint'):
    return f'''timing () {{ timing_type : {kind};
      {a} (grid) {{ index_1 ("0.002, 0.1"); index_2 ("0.007, 0.52");
        values ("-0.1, 0.2", \\
                "0.3, 0.4"); }}
      {b} (grid) {{ index_1 ("0.002, 0.1"); index_2 ("0.007, 0.52");
        values ("1.0, 2.0", "3.0, 4.5"); }}
    }}'''


def library():
    return '''library (example) {
      time_unit : "1ns"; capacitive_load_unit (1, pf);
      nom_voltage : 1.6; nom_temperature : 100;
      default_operating_conditions : SLOW;
      cell (example) {
      bus (addr) { pin (addr[0]) { direction : input; }
    ''' + arc('setup_rising') + arc('hold_rising') + '''}
      pin (rstb) {''' + arc('hold_rising') + '''}
      pin (clk) {''' + arc('minimum_period') + arc('min_pulse_width') + '''}
      bus (dout) {''' + arc('rising_edge', 'cell_rise', 'cell_fall') + '''}
      } }'''


class TimingExtraction(unittest.TestCase):
    def test_both_transitions_and_all_rows_including_negative_values(self):
        result = extract_timing(library(), 'example')
        self.assertEqual(result['setup']['addr']['rise']['min'], -0.1)
        self.assertEqual(result['setup']['addr']['rise']['max'], 0.4)
        self.assertEqual(result['hold']['rstb']['fall']['max'], 4.5)
        self.assertEqual(result['clk_q']['fall']['max'], 4.5)
        self.assertEqual(result['setup']['addr']['rise']['example'], -0.1)
        self.assertEqual(result['voltage'], 1.6)
        self.assertEqual(result['temperature'], 100)
        self.assertIn('SLOW', result['corner_label'])
        self.assertEqual(result['example_conditions']['output_load_pf'], 0.007)

    def test_rejects_wrong_units_missing_cell_and_incomplete_data(self):
        for text, name in [(library().replace('1ns', '1ps'), 'example'),
                           (library().replace('(1, pf)', '(1, ff)'), 'example'),
                           (library(), 'different'),
                           (library().replace('cell_fall', 'fall_transition'), 'example'),
                           (library()[:-2], 'example')]:
            with self.assertRaises(ValueError):
                extract_timing(text, name)


if __name__ == '__main__':
    unittest.main()
