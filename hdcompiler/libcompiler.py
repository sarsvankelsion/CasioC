#modified by hieuxyz(comment,supported by casio2k9) last modified at 12:09 AM 11-22-2024(GMT+7)
from colorama import init, Fore, Back, Style
import re
import sys
from text import char_to_hex  # Import char_to_hex từ file text.py
from functools import lru_cache

init()

# max_call_adr = 0x1ffff
max_call_adr = 0xfffff

def set_font(font_):
    global font, font_assoc
    font = font_
    font_assoc = dict((c, i) for i, c in enumerate(font))

def from_font(st):
    return [font_assoc[char] for char in st]

def to_font(charcodes):
    return ''.join(font[charcode] for charcode in charcodes)

def set_npress_array(npress_):
    global npress
    npress = npress_

def set_symbolrepr(symbolrepr_):
    global symbolrepr
    symbolrepr = symbolrepr_

@lru_cache(maxsize=256)
def byte_to_key(byte):
    if byte == 0:
        return '<NUL>'

    # TODO hack for classwiz without unstable
    sym = symbolrepr[byte]
    return f'<{byte:02x}>' if sym in ('@', '') else sym

    offset = 0
    sym = symbolrepr[byte]
    while byte and npress[byte] >= 100:
        byte = byte - 1
        offset += 1
    typesym = symbolrepr[byte] if byte else 'NUL'

    if set(sym) & set('\'"<>:'):
        sym = repr(sym)
    if set(typesym) & set('\'"<>:+'):
        typesym = repr(typesym)

    if offset == 0:
        return sym
    else:
        return f'<{sym}:{typesym}+{offset}>'

def get_npress(charcodes):
    if isinstance(charcodes, int):
        charcodes = (charcodes,)
    return sum(npress[charcode] for charcode in charcodes)

def get_npress_adr(adrs):
    if isinstance(adrs, int):
        adrs = (adrs,)
    assert all(0 <= adr <= max_call_adr for adr in adrs)
    return sum(get_npress((adr & 0xFF, (adr >> 8) & 0xFF)) for adr in adrs)

def optimize_adr_for_npress(adr):
    '''
    For a 'POP PC' command, the lowest significant bit in the address
    does not matter. This function use that fact to minimize number
    of key strokes used to enter the hackstring.
    '''
    return min((adr, adr ^ 1), key=get_npress_adr)

def optimize_sum_for_npress(total):
    ''' Return (a, b) such that a + b == total. '''
    return ['0x' + hex(x)[2:].zfill(4) for x in min(
        ((x, (total - x) % 0x10000) for x in range(0x0101, 0x10000)),
        key=get_npress_adr
    )]

def note(st):
    ''' Print st to stderr. Used for additional information (note, warning) '''
    sys.stderr.write(st)

def to_lowercase(s):
    return s.lower()

def canonicalize(st):
    ''' Make (st) canonical. '''
    #st = st.lower()
    st = st.strip()
    # remove spaces around non alphanumeric
    st = re.sub(r' *([^a-z0-9]) *', r'\1', st)
    return st

def del_inline_comment(line):
    return (line + '#')[:line.find('#')].rstrip()

def add_command(command_dict, address, command, tags, debug_info=''):
    ''' Add a command to command_dict. '''
    assert command, f'Empty command {debug_info}'
    assert type(command_dict) is dict

    for disallowed_prefix in '0x', 'call', 'goto', 'adr_of':
        assert not command.startswith(disallowed_prefix), \
            f'Command ends with "{disallowed_prefix}" {debug_info}'
    assert not command.endswith(':'), \
        f'Command ends with ":" {debug_info}'
    assert ';' not in command, \
        f'Command contains ";" {debug_info}'

    # this is inefficient
    for prev_command, (prev_adr, prev_tags) in command_dict.items():
        if prev_command == command or prev_adr == address:
            assert False, f'Command appears twice - ' \
                f'first: {prev_command} -> {prev_adr:05X} {prev_tags}, ' \
                f'second: {command} -> {address:05X} {tags} - ' \
                f'{debug_info}'

    command_dict[command] = (address, tuple(tags))

# A dict of {name: (address, tags)} to append result to.
commands = {}
datalabels = {}

def get_commands(filename):
    ''' Read a list of gadget names.

    Args:
        A dict
    '''
    global commands
    with open(filename, 'r', encoding='u8') as f:
        data = f.read().splitlines()

    in_comment = False
    line_regex = re.compile(r'([0-9a-fA-F]+)\s+(.+)')
    for line_index0, line in enumerate(data):
        line = line.strip()

        # multi-line comments
        if line == '/*':
            in_comment = True
            continue
        if line == '*/':
            in_comment = False
            continue
        if in_comment:
            continue

        line = del_inline_comment(line)
        if not line:
            continue

        match = line_regex.fullmatch(line)
        address, command = match[1], match[2]

        command = canonicalize(command)
        command = to_lowercase(command)

        tags = []  # process tags (leading `{...}`)
        while command and command[0] == '{':
            i = command.find('}')
            if i < 0:
                raise Exception(f'Line {line_index0 + 1} '
                                'has unmatched "{"')
            tags.append(command[1:i])
            command = command[i + 1:]

        try:
            address = int(address, 16)
        except ValueError:
            raise Exception(f'Line {line_index0 + 1} has invalid address: {address!r}')

        add_command(commands, address, command, tags, f'at {filename}:{line_index0 + 1}')

def get_disassembly(filename):
    '''Try to parse a disassembly file with annotated address.

    Each line should look like this:

        mov r2, 1                      ; 0A0A2 | 0201
    '''
    global disasm
    with open(filename, 'r', encoding='u8') as f:
        data = f.read().splitlines()

    line_regex = re.compile(r'\t(.*?)\s*; ([0-9a-fA-F]*) \|')
    disasm = []
    for line in data:
        match = line_regex.match(line)  # match prefix
        if match:
            addr = int(match[2], 16)
            while addr >= len(disasm):
                disasm.append('')
            disasm[addr] = match[1]

def read_rename_list(filename):
    '''Try to parse a rename list.

    If the rename list is ambiguous without disassembly, it raises an error.
    '''
    global commands, datalabels
    with open(filename, 'r', encoding='u8') as f:
        data = f.read().splitlines()

    line_regex = re.compile(r'^\s*([\w_.]+)\s+([\w_.]+)')
    global_regex = re.compile(r'f_([0-9a-fA-F]+)')
    local_regex = re.compile(r'.l_([0-9a-fA-F]+)')
    data_regex = re.compile(r'd_([0-9a-fA-F]+)')
    hexadecimal = re.compile(r'[0-9a-fA-F]+')

    last_global_label = None
    for line_index0, line in enumerate(data):
        match = line_regex.match(line)
        if not match:
            continue
        raw, real = match[1], match[2]
        if real.startswith('.'):
            # we don't get local labels.
            continue

        match = data_regex.fullmatch(raw)
        if match:
            addr = int(match[1], 16)
            datalabels[real] = addr
            continue

        addr = None
        if hexadecimal.fullmatch(raw):
            addr = int(raw, 16)
            last_global_label = None
            # because we don't know whether this label is global or local
        else:
            match = global_regex.match(raw)
            if match:
                addr = int(match[1], 16)
                if len(match[0]) == len(raw):  # global_regex.fullmatch
                    last_global_label = addr
                else:
                    match = local_regex.fullmatch(raw[len(match[0]):])
                    if match:  # full address f_12345.l_67
                        addr += int(match[1], 16)
            else:
                match = local_regex.fullmatch(raw)
                if match:
                    if last_global_label is None:
                        print('Label cannot be read: ', line)
                        continue
                    else:
                        addr = last_global_label + int(match[1], 16)

        if addr is not None:
            assert addr < len(disasm), f'{addr:05X}'
            if disasm[addr].startswith('push lr'):
                tags = 'del lr',
                addr += 2
            else:
                tags = 'rt',
                a1 = addr + 2
                while not any(disasm[a1].startswith(x) for x in ('push lr', 'pop pc', 'rt')):
                    a1 += 2
                if not disasm[a1].startswith('rt'):
                    tags = tags + ('del lr',)

            if real in commands:
                if 'override rename list' in commands[real][1]:
                    continue
                if commands[real] == (addr, tags):
                    note(f'Warning: Duplicated command {real}\n')
                    continue

            add_command(commands, addr, real, tags=tags,
                       debug_info=f'at {filename}:{line_index0 + 1}')
        else:
            raise ValueError('Invalid line: ' + repr(line))

def sizeof_register(reg_name):
    # assume reg_name is a valid register name
    return {'r': 1, 'e': 2, 'x': 4, 'q': 8}[reg_name[0]]

result = []  # list of ints in range 0..255
labels = {}
adr_of_cmds = []  # list of (source adr, offset, target label)
adr_arith_cmds = []  # list of deferred adr_arith commands
pr_length_cmds = []  # list of deferred pr_length commands
endaddr = ""
end = []
addr = 1
# Right after the buffer overflow, the memory region [home..home+len(result)[
# should have value = result (after replacing labels)
home = None
in_comment  = False
# Dictionary to store string variables and their hex values
string_vars = {}
        
def process(line):
    # the processing result will affect those variables
    global result, labels, adr_of_cmds, adr_arith_cmds, pr_length_cmds, home, string_vars, in_comment, endaddr, end


    if not line or line.isspace():
        return

    if line.startswith('/*'):
        in_comment = True
        return
        
    # Kiểm tra kết thúc comment    
    if '*/' in line:
        in_comment = False
        return
        
    # Nếu đang trong comment thì bỏ qua xử lý
    if in_comment:
        return

    elif ';' in line:
        ''' Compound statement. Syntax:
        `<statement1> ; <statement2> ; ...`
        '''
        for command in line.split(';'):
            process(to_lowercase(command)),

    elif line.strip() and line.strip()[-1] == ':':
        ''' Syntax: `<label>:`
        Special: If the label is 'home', it specifies the point to
        start program execution. By default it's at the begin.
        '''
        label = to_lowercase(line[:-1])
        assert label not in labels, f'Duplicated label: {label}'
        labels[label] = len(result)

    elif line.startswith('0x'):
        if '+' in line:
            ''' Syntax: `0x<hexadecimal digits>+<dec>` '''
            hex_part, dec_part = line.split('+')
            initial_value = int(hex_part, 16) 
            increment = int(dec_part)
            result_value = initial_value + increment
            n_byte = len(hex_part) // 2 - 1
            for _ in range(n_byte):
                result.append(result_value & 0xFF)
                result_value >>= 8
        elif '-' in line:
            ''' Syntax: `0x<hexadecimal digits>-<dec>` '''
            hex_part, dec_part = line.split('-')
            initial_value = int(hex_part, 16)
            decrement = int(dec_part)
            result_value = initial_value - decrement
            n_byte = len(hex_part) // 2 - 1
            for _ in range(n_byte):
                result.append(result_value & 0xFF)
                result_value >>= 8
        else:
            ''' Syntax: `0x<hexadecimal digits>` '''
            assert len(line) % 2 == 0, f'Invalid data length'
            n_byte = len(line) // 2 - 1
            data = int(line, 16)
            for _ in range(n_byte):
                result.append(data & 0xFF)
                data >>= 8
    elif line.startswith('hex') and 'hex_' not in line:
        data = line[3:].strip()
        assert len(data.replace(" ", "")) % 2 == 0, f'Invalid data length'
        data_bytes = bytes.fromhex(data)
        result.extend(data_bytes)

    elif line.startswith('call'):
        ''' Syntax: `call <address>` or `call <built-in>`. '''
        try:
            adr = int(line[4:], 16)
        except ValueError:
            #adr, tags = commands[to_lowercase(line[4:].strip())]
            adr, tags = commands[line[4:].strip()]
            for tag in tags:
                if tag.startswith('warning'):
                    note(tag + '\n')

        assert 0 <= adr <= max_call_adr, f'Invalid address: {adr}'
        adr = optimize_adr_for_npress(adr)
        process(f'0x{adr + 0x30300000:0{8}x}')

    elif line.startswith('goto'):
        ''' Syntax: `goto <label>` '''
        label = to_lowercase(line[4:])
        process(f'er6 = adr_of [-2] {label}')
        process('call sp=er6,pop er8')

    elif line.startswith('adr_of'):
        ''' Syntax: `adr_of [offset] <label>` | `adr_of <label>` '''
        line = to_lowercase(line[6:].strip())
        if line[0] == '[':
            i = line.index(']')
            if '->' in line:
                addr1, addr2 = line[1:i].strip().split('->')
                addr1 = int(addr1, 16)
                addr2 = int(addr2, 16)
                if addr2 > addr1:
                    offset = addr1 - addr2
                else:
                    offset = addr2 - addr1
                label = line[i + 1:].strip()
            else:
                offset = int(line[1:i], 0)
                label = line[i + 1:].strip()
        else:
            offset = 0
            label = line.strip()

        adr_of_cmds.append((len(result), offset, label))
        result.extend((0, 0))

    elif line in datalabels:
        ''' `<label>`. '''
        process(f'{line}+0')

    elif '+' in line and line[:line.find('+')] in datalabels:
        ''' `<label> + <offset>`. '''
        label, offset = line.split('+')
        process(f'0x{datalabels[label] + int(offset, 0):04x}')

    elif line in commands:
        ''' `<built-in>`. Equivalent to `call <built-in>`. '''
        line = to_lowercase(line)
        process('call ' + line)

    elif '=' in line:
        ''' Syntax:
        `register = <value> [, adr_of <label> [, ...]]`
        '''
        i = line.index('=')
        register, value = line[:i], line[i + 1:].lstrip()

        assert '=' not in value, f'Thiếu byte trong thanh ghi nè ba nội : {line}'
        value = value.replace(',', ';')

        process(f'call pop {register}')

        l1 = len(result)
        process(value)
        assert len(result) - l1 == sizeof_register(register), \
            f'Line {line!r} source/destination target mismatches'

    elif line[0] == '$':
        ''' Python eval. The result will be processed as commands. '''
        x = eval(line[1:])
        if isinstance(x, str):
            process(x)
        elif isinstance(x, list) or isinstance(x, tuple):
            for command in x:
                process(command)

    elif line.startswith('org'):
        ''' Syntax: `org <expr>`

        Specify the address of this location after mapping.
        Only use this for loader mode.
        '''
        global hx
        hx = eval(line[3:])
        home1 = hx - len(result)
        assert home is None or home == home1, 'Inconsistent value of `home`'
        home = home1
    elif line.startswith('backup is '):
        global backup
        backup = line.replace("backup is ", "").strip()
    elif line.startswith('addrcopy is '):
        global addrcopy
        addrcopy = line.replace("addrcopy is ", "").strip()
    elif line.startswith('loop880'):
        heh = f"""set_segment:
setlr
di,rt
call pop xr0 
adr_of length
0x0001
[er0]=er2,rt
loop:
call pop qr0
adr_of {addrcopy}
{backup}
pr_length
adr_of [-2] {addrcopy}
hex e6 4d
length:
remaining_length
0x0000
call sp=er6,pop er8"""
        huh = heh.splitlines()
        for i in huh:
            process(i)
    elif line.startswith('loop580'):
        heh = f"""set_segment:
setlr
di,rt
call pop xr0 
adr_of length
0x0001
[er0]=er2,rt
call pop qr0
pr_length
{backup}
adr_of {addrcopy}
adr_of [-2] {addrcopy}
0x8932
length:
remaining_length
0x0000
sp=er6,pop er8"""
        huh = heh.splitlines()
        for i in huh:
            process(i)
    elif line.startswith('backup580'):
        heh = f"""backup:
call pop xr0
{backup}
adr_of {addrcopy}
call 0x09450
pr_length
"""
        huh = heh.splitlines()
        for i in huh:
            print(i)
            process(i)
    elif line.startswith('backup880'):
        heh = f"""backup:
call pop xr0
{backup}
adr_of {addrcopy}
call 0x14DE8
pr_length
0x0000
"""
        huh = heh.splitlines()
        for i in huh:
            process(i)
    elif line.startswith('adr_arith'):
        ''' Syntax: `adr_arith [offset] <label> - adr_arith [offset] <label>`
        '''
        
        # Tìm vị trí của dấu - cuối cùng để tách chuỗi
        last_minus_pos = len(line) - 1
        while last_minus_pos > 0:
            if line[last_minus_pos] == '-' and 'adr_arith' in line[last_minus_pos:]:
                break
            last_minus_pos -= 1
        
        if last_minus_pos <= 0:
            raise ValueError(f'Mày tính độ dài mà syntax còn sai :v: {line}')
            
        left_part = line[9:last_minus_pos].strip()  # Bỏ 'adr_arith' đầu tiên
        right_part = line[last_minus_pos + 1:].strip()
        
        def parse_adr_part(part):
            part = part.strip()
            if part.startswith('adr_arith'):
                part = part[9:].strip()
    
            offset = 0
    
            # Kiểm tra xem có offset không
            if '[' in part and ']' in part:
                try:
                    start_idx = part.index('[')
                    end_idx = part.index(']')
                    offset_str = part[start_idx+1:end_idx].strip()
                    # Hỗ trợ số âm trong offset
                    try:
                        offset = int(offset_str, 0)
                    except ValueError:
                        # Thử parse lại nếu có dấu - ở đầu
                        if offset_str.startswith('-'):
                            offset = -int(offset_str[1:], 0)
                        else:
                            raise
                            
                    label = part[end_idx+1:].strip()  # Lấy label sau dấu ]
                    if not label:  # Label là bắt buộc
                        raise ValueError(f'Ai gặm mất label rồi kìa :D : {part}')
                except ValueError as e:
                    if str(e).startswith('Missing'):
                        raise e
                    raise ValueError(f'Invalid offset in: {part}')
            else:
                # Trường hợp không có offset
                label = part.strip()
                if not label:  # Label là bắt buộc
                    raise ValueError(f'Thiếu label này má: {part}')
    
            return offset, label
    
        # Parse both parts
        left_offset, left_label = parse_adr_part(left_part)
        right_offset, right_label = parse_adr_part(right_part)
    
        # Store the deferred command
        adr_arith_cmds.append((len(result), left_offset, left_label, right_offset, right_label))
        result.append(0)  # Placeholder for the result

    elif line.startswith('pr_length'):
        ''' Syntax: `pr_length`
        Defers the calculation of the program length until the end of processing.
        '''
        pr_length_cmds.append(len(result))
        result.extend((0, 0))  # Placeholder for the length
    elif line.startswith('remaining_length'):
        endaddr = len(result)
        end.append(len(result))
        result.extend((0, 0))

    elif line.startswith('str'):
        ''' Syntax:
        - str <var> "<string>" :        store the hex value of the string into the variable <var> -> Define and store string in variable
        - str <var>           :        Use(get value) of <var> [requires declaring string of var before calling] -> Use previously defined string variable
        - str "<string>"       :        Get the hex value of a <string> directly (without using a variable, or reusing it) -> Direct string usage
        + char "~" converted to space
        + support for 580vnx
        '''
        content = line[3:].strip()

        def string_to_bytes(text):
            byte_list = []
            print("In văn bản nè :D :", text.replace('~', ' '))
            #print("Processing string:", text)   # Debug print
            for c in text:
                try:
                    hex_val = char_to_hex[c]
                    #print(f"Character '{c}' -> hex value: {hex_val}")  # Debug print
                    if len(hex_val) == 2:
                        byte_list.append(int(hex_val, 16))
                    elif len(hex_val) == 4:
                        byte_list.extend([int(hex_val[:2], 16), int(hex_val[2:], 16)])
                except KeyError:
                    raise ValueError(f"Character '{c}' not found in conversion table")
            #print("Final byte list:", byte_list)  # Debug print
            return byte_list

        if '"' in content:
            quote_pos = content.find('"')
            var_name = content[:quote_pos].strip() if quote_pos > 0 else None
            text = content[quote_pos+1:].rstrip('"')

            if var_name:
                string_vars[var_name] = text
            else:
                bytes_list = string_to_bytes(text)
                #print("Final bytes to add:", bytes_list)  # Debug print
                result.extend(bytes_list)

        elif content:
            var_name = content.strip()
            if var_name in string_vars:
                text = string_vars[var_name]
                bytes_list = string_to_bytes(text)
                #print("Final bytes to add:", bytes_list)  # Debug print
                result.extend(bytes_list)
            else:
                raise ValueError(f"Undefined string variable: {var_name}")
        else:
            raise ValueError("Invalid str command syntax")

    else:
        assert False, f'Có lệnh này đâu má :v'

def finish_processing():
    global result, labels, adr_arith_cmds, pr_length_cmds, addr

    # Process deferred adr_arith commands
    for pos, left_offset, left_label, right_offset, right_label in adr_arith_cmds:
        if left_label in labels and right_label in labels:
            left_adr = labels[left_label] + left_offset
            right_adr = labels[right_label] + right_offset
            result_adr = (left_adr - right_adr) & 0xFF
            result[pos] = result_adr
        else:
            raise ValueError(f'Làm gì có label này má ơi: {left_label if left_label not in labels else right_label}')

    # Process deferred pr_length commands
    for pos in pr_length_cmds:
        pr_length = len(result)
        result[pos] = pr_length & 0xFF
        result[pos + 1] = (pr_length >> 8) & 0xFF
    addr = hx + len(result)
    for pos in end:
        remaining = len(result) - endaddr
        result[pos] = remaining & 0xFF
        result[pos +1] = (remaining >> 8) & 0xFF
def process_program(args, program, overflow_initial_sp):
    '''
    Take a program (list of command lines) and print the compiled program
    to the console.
    '''
    global result, labels, adr_of_cmds, adr_arith_cmds, pr_length_cmds, home, addr

    # Read the entire program
    modified_program = []
    setup_loop_detected = False

    for input_line in program:
        line = canonicalize(del_inline_comment(input_line))
        if line.startswith('setup_loop'):
            setup_loop_detected = True
            parts = line.split(',')
            if len(parts) == 2:  # Chính xác 2 phần
                src = parts[0].split()[1].strip()
                src_backup = parts[1].strip()
                label = 'home'  # Mặc định
            elif len(parts) == 3:  # Chính xác 3 phần 
                src = parts[0].split()[1].strip()
                src_backup = parts[1].strip()
                label = parts[2].strip() if parts[2].strip() != 'None' else 'home'
            else:
                raise ValueError(f'Invalid setup_loop directive: {line} - Expected 2 or 3 parts')
        
            # Replace the setup_loop line with the provided code snippet
            modified_code = f"""
restore:
    setlr
    DI,RT
    xr0 = adr_of length, 0x01, 0x00
    [er0] = er2,rt
    qr0 = pr_length, {src_backup}, {src}, 0x0000
    0x8932
length:
    0x0800
    0x0000
set_sp:
    er6 = adr_of [-2] {label}
    sp = er6,pop er8
"""
            modified_program.extend(modified_code.strip().split('\n'))
        else:
            modified_program.append(line)

    # If setup_loop was not detected, use the original program
    if not setup_loop_detected:
        modified_program = program

    # Process the modified program
    for line in modified_program:
        line = canonicalize(del_inline_comment(line))

        # Chỉ sử dụng to_lowercase nếu dòng không bắt đầu bằng "str"
        if not line.lower().startswith("str"):
            line = to_lowercase(line)

        # temporarily redirect notes to note_log
        note_log = ''
        global note
        note_ = note

        def note(st):
            nonlocal note_log
            note_log += st

        old_len_result = len(result)
        try:
            process(line)
        except:
            note_(f'Trong lúc tao đang chạy dòng \n{line}\n')
            raise

        # labels have undetermined value and they are temporarily represented
        # by zeroes in result list
        if args.format == 'key' and \
                any(x != 0 and get_npress(x) > 100 for x in result[old_len_result:]):
            note('Line generates many keypresses\n')

        # restore warnings
        note = note_
        if note_log:
            note(f'Trong lúc tao đang chạy dòng\n{line}\n')
            note(note_log)

    # Process deferred adr_arith and pr_length commands
    finish_processing()

    # A list of (source adr, offset relative to `home`).
    adr_of_cmds = [(source_adr, labels[target_label] + offset)
                   for source_adr, offset, target_label in adr_of_cmds]

    if args.target in ('none', 'overflow'):
        if args.target == 'overflow':
            assert len(result) <= 100, 'Program too long'

        if home is None:  # `org` is not used
            # compute value of `home`
            home = overflow_initial_sp
            if 'home' in labels:
                home -= labels['home']  # so that the SP starts at the `home:` label
            if home + len(result) > 0x8E00:
                note(f'Warning: Program length after home = {len(result)} bytes'
                     f' > {0x8E00 - home} bytes\n')

            min_home = home
            while min_home >= 0x8154 + 200:
                min_home -= 100
            while home + len(result) <= 0x8E00:
                home += 100  # 0x8E00: end of RAM
            home = min(range(min_home, home, 100), key=lambda home:
                        (
                            sum(  # count number of ... satisfy condition
                                get_npress_adr(home + home_offset) >= 100
                                for source_adr, home_offset in adr_of_cmds),
                            -home  # if ties then take max `home`
                        )
                        )

    elif args.target == 'loader':
        if home is None:
            home = 0x85b0 - len(result)
            entry = home + labels.get('home', 0) - 2
            result.extend((0x6a, 0x4f, 0, 0, entry & 255, entry >> 8, 0x68, 0x4f, 0, 0))
            while home + len(result) < 0x85d7:
                result.append(0)
            result.extend((0xff, 0xae, 0x85))
            home2 = 0
            assert (home - home2) >= 0x8501, 'Program too long'
            while get_npress_adr(home - home2) >= 100:
                home2 += 1

    else:
        assert False, 'Internal error'

    # home is picked now, now substitute in the result
    assert home is not None
    for source_adr, home_offset in adr_of_cmds:
        target_adr = home + home_offset
        assert result[source_adr] == 0
        result[source_adr] = target_adr & 0xFF
        assert result[source_adr + 1] == 0
        result[source_adr + 1] = target_adr >> 8

    # debug print label location
    for label, home_offset in labels.items():
        note(f'Đoạn lệnh {label} đang bị tao bắt cóc ở {home + home_offset:04X}.Vui lòng nộp tiền chuộc :D\n')

    if args.target == 'overflow':
        # scroll it around (use the most inefficient way)
        hackstring = list(map(ord, '1234567890' * 10))  # but still O(n)
        for home_offset, byte in enumerate(result):
            assert isinstance(byte, int), (home_offset, byte)
            hackstring[(home + home_offset - 0x8154) % 100] = byte

    # done
    if args.target == 'overflow' and args.format == 'hex':
        print(''.join(f'{byte:0{2}x}' for byte in hackstring))
    elif args.target == 'none' and args.format == 'hex':
        print(Fore.GREEN + f'\t\t\t\t===\t0x%04x -> {hex(addr)}\t===\n{Style.RESET_ALL}' % home, *map('%02x'.__mod__, result))
    elif args.target == 'none' and args.format == 'key':
        print(f'{home:#06x}:', ' '.join(
            byte_to_key(byte) for byte in result
        ))
    elif args.target == 'loader' and args.format == 'key':
        # NOTE: loader target may be specific to 570es+/991es+
        print('Address to load: %s %s' % (byte_to_key((home - home2) & 255), byte_to_key((home - home2) >> 8)))
        for i in range(home2):
            result.insert(0, 0)
        import keypairs
        print(keypairs.format(result))
    elif args.target == 'overflow' and args.format == 'key':
        print(' '.join(byte_to_key(x) for x in hackstring))
    else:
        raise ValueError('Unsupported target/format combination')

rom = None

def get_rom(x):
    global rom

    if isinstance(x, str):
        with open(x, 'rb') as f:
            rom = f.read()
    elif isinstance(x, bytes):
        rom = x
    else:
        raise TypeError

def find_equivalent_addresses(rom: bytes, q: set):
    # handles BL / POP PC, BC AL, B
    from collections import defaultdict
    comefrom = defaultdict(list)  # adr -> list of comefrom addresses

    for i in range(0, len(rom), 2):  # BC AL
        if rom[i + 1] == 0xce:
            offset = rom[i]
            if offset >= 128:
                offset -= 256
            comefrom[i >> 16 | ((i + (offset + 1) * 2) & 0xffff)].append(i)

    for i in range(0, len(rom) - 2, 2):  # B
        if (
                rom[i] == 0x00 and
                (rom[i + 1] & 0xf0) == 0xf0):
            comefrom[(rom[i + 1] & 0x0f) << 16 | rom[i + 3] << 8 | rom[i + 2]].append(i)

    for i in range(0, len(rom) - 4, 2):  # BL / POP PC
        if (
                rom[i] == 0x01 and
                (rom[i + 1] & 0xf0) == 0xf0 and
                (rom[i + 4] & 0xf0) == 0x8e and
                (rom[i + 5] & 0xf0) == 0xf2):
            comefrom[(rom[i + 1] & 0x0f) << 16 | rom[i + 3] << 8 | rom[i + 2]].append(i)

    ans = set()
    while q:
        adr = q.pop()
        if adr in ans:
            continue
        ans.add(adr)

        if adr in comefrom:
            q.update(comefrom[adr])

    return ans

def optimize_gadget_f(rom: bytes, gadget: bytes) -> set:
    assert len(gadget) % 2 == 0
    q = set()  # pending addresses

    # find occurrences of gadget in rom
    for i in range(0, len(rom) - len(gadget) + 1, 2):
        if rom[i:i + len(gadget)] == gadget:
            q.add(i)

    return find_equivalent_addresses(rom, q)

def optimize_gadget(gadget: bytes) -> set:
    global rom
    return optimize_gadget_f(rom, gadget)

# helper function for printing gadget addresses
def print_addresses(adrs, n_preview: int):
    adrs = list(map(optimize_adr_for_npress, adrs))
    for adr in sorted(adrs, key=get_npress_adr):
        keys = ' '.join(map(byte_to_key,
                            (adr & 0xff, (adr >> 8) & 0xff, 0x30 | adr >> 16)
                            ))
        print(f'{adr:05x}  {get_npress_adr(adr):3}    {keys:20}')

        i = adr & 0xffffe
        for _ in range(n_preview):
            print(' ' * 4 + disasm[i])
            i += 2
            while i < len(disasm) and not disasm[i]:
                i += 2
            if i >= len(disasm):
                break