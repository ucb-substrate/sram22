"""Read timing groups structurally; retain both transitions and all table entries."""
from dataclasses import dataclass, field
import re

TOKEN = re.compile(r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\\\s*\n|[{}():;,]|[^\s{}():;,]+', re.S)

@dataclass
class Group:
    kind: str
    args: list
    attrs: dict = field(default_factory=dict)
    children: list = field(default_factory=list)


def parse(text):
    tokens = [t for t in TOKEN.findall(text)
              if not t.startswith(('/*', '//')) and not (t.startswith('\\') and t[1:].isspace())]
    pos = 0
    def read_until(end):
        nonlocal pos
        values = []
        while pos < len(tokens) and tokens[pos] != end:
            token = tokens[pos]
            if token != ',':
                values.append(token[1:-1] if token.startswith('"') else token)
            pos += 1
        if pos == len(tokens): raise ValueError(f'Unterminated Liberty statement, expected {end}')
        pos += 1
        return values
    def body(group, nested=False):
        nonlocal pos
        while pos < len(tokens):
            name = tokens[pos]; pos += 1
            if name == '}': return
            if pos >= len(tokens): raise ValueError('Incomplete Liberty statement')
            kind = tokens[pos]; pos += 1
            if kind == ':':
                group.attrs[name] = read_until(';')
            elif kind == '(':
                args = read_until(')')
                if pos >= len(tokens): raise ValueError('Incomplete Liberty group')
                delim = tokens[pos]; pos += 1
                if delim == '{':
                    child = Group(name, args); body(child, True); group.children.append(child)
                elif delim == ';': group.attrs[name] = args
                else: raise ValueError(f'Unexpected Liberty token {delim}')
            else: raise ValueError(f'Unexpected Liberty token {kind}')
        if nested: raise ValueError('Unterminated Liberty group')
    root = Group('root', [])
    body(root)
    return root


def numeric(values):
    return [float(v) for value in values for v in re.split(r'[,\s]+', value.strip()) if v]


def summarize(tables):
    values = [x for t in tables for x in numeric(t.attrs['values'])]
    if not values: raise ValueError('Empty timing table')
    return {'min': min(values), 'max': max(values), 'example': values[0]}


def extract_timing(text, macro):
    libraries = [g for g in parse(text).children if g.kind == 'library']
    if len(libraries) != 1: raise ValueError('Expected one Liberty library')
    lib = libraries[0]
    if lib.attrs.get('time_unit') != ['1ns']: raise ValueError('Expected Liberty time_unit 1ns')
    if lib.attrs.get('capacitive_load_unit') != ['1', 'pf']: raise ValueError('Expected Liberty load unit 1pf')
    cells = [g for g in lib.children if g.kind == 'cell' and g.args == [macro]]
    if len(cells) != 1: raise ValueError(f'Missing/ambiguous Liberty cell {macro}')
    voltage = float(lib.attrs['nom_voltage'][0]); temperature = float(lib.attrs['nom_temperature'][0])
    conditions = lib.attrs.get('default_operating_conditions', ['unspecified'])[0]
    result = {'macro': macro, 'corner_label': f'{voltage:g} V, {temperature:g} °C ({conditions})',
              'voltage': voltage, 'temperature': temperature, 'operating_conditions': conditions,
              'time_unit': 'ns', 'setup': {}, 'hold': {}}
    constraints = {}
    def visit(group, pin=None):
        if group.kind in ('pin', 'bus'): pin = group.args[0]
        if group.kind == 'timing' and pin:
            typ = group.attrs.get('timing_type', [''])[0]
            base = re.sub(r'\[\d+\]$', '', pin)
            for table in group.children:
                if table.kind in ('rise_constraint', 'fall_constraint', 'cell_rise', 'cell_fall'):
                    constraints.setdefault((base, typ, table.kind), []).append(table)
        for child in group.children: visit(child, pin)
    visit(cells[0])
    for (pin, typ, table), tables in constraints.items():
        transition = 'rise' if table.startswith('rise') or table == 'cell_rise' else 'fall'
        stats = summarize(tables)
        if typ in ('setup_rising', 'hold_rising'):
            result[typ.split('_')[0]].setdefault(pin, {})[transition] = stats
        elif pin == 'clk' and typ in ('minimum_period', 'min_pulse_width'):
            result.setdefault(typ, {})[transition] = stats
        elif pin == 'dout' and typ == 'rising_edge':
            result.setdefault('clk_q', {})[transition] = stats
    if 'addr' not in result['setup'] or 'addr' not in result['hold']:
        raise ValueError('Missing required address/output/clock timing groups')
    for category in ('clk_q', 'minimum_period', 'min_pulse_width'):
        if set(result.get(category, {})) != {'rise', 'fall'}:
            raise ValueError(f'Missing rise/fall tables for {category}')
    for category in ('setup', 'hold'):
        for pin, entry in result[category].items():
            if set(entry) != {'rise', 'fall'}:
                raise ValueError(f'Missing rise/fall tables for {pin} {category}')
    addr = constraints[('addr', 'setup_rising', 'rise_constraint')][0]
    cq = constraints[('dout', 'rising_edge', 'cell_rise')][0]
    result['example_conditions'] = {
        'setup_index_1_ns': numeric(addr.attrs['index_1'])[0],
        'setup_index_2_ns': numeric(addr.attrs['index_2'])[0],
        'clock_slew_ns': numeric(cq.attrs['index_1'])[0],
        'output_load_pf': numeric(cq.attrs['index_2'])[0],
    }
    return result
