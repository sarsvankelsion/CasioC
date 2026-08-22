## Compiler and Tool Usage Guide

This guide provides detailed instructions on how to use the compiler and associated tools to process and compile code. The syntax and usage examples are based on the provided `compiler.py` and `libcompiler.py` scripts.

### Table of Contents
1. [Introduction](#introduction)
2. [Syntax Overview](#syntax-overview)
3. [Compiler Usage](#compiler-usage)
4. [Examples](#examples)
5. [Additional Notes](#additional-notes)
6. [Code Structure](#code-structure)

### Introduction

The compiler script (`compiler.py`) and the library (`libcompiler.py`) work together to process and compile code. The compiler reads various input files, processes the code, and outputs the compiled result. The library provides essential functions for handling fonts, symbols, and other necessary operations.

### Syntax Overview

The syntax for the code processed by the compiler is as follows:

#### Commands

- **Comment or multi comment line**: Used to note or ignore so that the compiler does not process it using the syntax `#<sth>` or `/*<sth>*/`(for multi comment line).
   ```plaintext
   # this is the comment line
   /* comment line 1
   comment line2
   ...
   */ #end multi comment line
   ```

- **Labels**: Define labels using the syntax `label:`. Labels can be used to mark specific points in the code.
  ```plaintext
  home:
  ```

- **Hexadecimal Data**: Insert hexadecimal data using the syntax `0x<hexadecimal digits>`.
  ```plaintext
  0x1234
  ```

- **Calls**: Make calls to addresses or built-in functions using the syntax `call <address>` or `call <built-in>`.
  ```plaintext
  call 0x1234
  call built_in_function
  ```

- **Goto**: Jump to a label using the syntax `goto <label>`.
  ```plaintext
  goto label
  ```

- **Address Of**: Get the address of a label using the syntax `adr_of [offset] <label>` or `adr_of <label>`.
  ```plaintext
  adr_of [10] label
  adr_of label
  ```

- **Register Assignment**: Assign values to registers using the syntax `register = <value> [, adr_of <label> [, ...]]`.
  ```plaintext
  r1 = 0x1234, adr_of label
  ```

- **Python Eval**: Evaluate Python expressions using the syntax `$<expression>`.
  ```plaintext
  $eval_expression
  ```

- **Org**: Specify the address of a location after mapping using the syntax `org <expr>`.
  ```plaintext
  org 0x1234
  ```

- **Address Arithmetic**: Perform address arithmetic using the syntax `adr_arith [offset] <label> - adr_arith [offset] <label>`.
  ```plaintext
  adr_arith [10] label1 - adr_arith [20] label2
  ```

- **Program Length**: Defer the calculation of the program length using the syntax `pr_length`.
  ```plaintext
  pr_length
  ```

- **String Handling**: Handle strings using the syntax `str <var> "<string>"`, `str <var>`, or `str "<string>"`.
  ```plaintext
  str var "Hello, World!"
  str var
  str "Hello, World!"
  ```

#### Compound Statements

- **Compound Statements**: Combine multiple statements using the syntax `<statement1> ; <statement2> ; ...`.
  ```plaintext
  call 0x1234 ; goto label
  ```

### Compiler Usage

To use the compiler, follow these steps:

1. **Prepare Input Files**: Ensure you have the necessary input files (provided):
   - `rom.bin`: The ROM binary file.
   - `disas.txt`: The disassembly file.
   - `gadgets`: The gadgets file.
   - `labels`: The labels file.
   - `../labels_sfr`: Additional labels file.

2. **Run the Compiler**: Execute the compiler script with the desired options. The script supports the following arguments:
   - `-t, --target`: Specify the target (default is `none`).
   - `-f, --format`: Specify the output format (`hex` or `key`, default is `key`).
   - `-g, --gadget-adr`: Specify the address of the gadget to optimize.
   - `-gb, --gadget-bin`: Specify the gadget in binary (big endian).
   - `-gn, --gadget-nword`: Specify the length of the gadget to optimize (inf if not provided).
   - `-p, --preview-count`: Specify the number of lines to preview (optimize gadget mode).

   Example command:
   ```sh
   python3 compiler.py -t none -f key -g 0x1234 -gb 0x4567 -gn 2 -p 10
   ```

3. **Run the Compiler with the available script**: We have created a script file to run the attached compiler (which is `580vnxcompiler.batfor Windows you can run 580vnxcompiler.bat`(for Windows) and `run.sh`(for Linux) for the casio fx580vnx model and it will receive your program file to be processed in the folder `580vnx_ropchain` and it will output in hex form(`-f hex`)
   Command(For Windows you can run `580vnxcompiler.bat` by double clicking on it):
   ```sh
   bash run.sh
   ```

4. **Process the Program**: The compiler will read the program from standard input, process it, and output the compiled result.

### Examples

#### Example 1: Simple Program

```plaintext
home:
    0x1234
    call 0x5678
    goto end
end:
```

#### Example 2: Using Labels and Address Of

```plaintext
start:
    adr_of label1
    goto label1
label1:
    0x9ABC
```

#### Example 3: String Handling

```plaintext
str var "Hello, World!"
str var
str "Goodbye, World!"
```

### Additional Notes

- **Font and Symbol Representation**: The compiler uses a custom font and symbol representation defined in the `FONT` and `symbols` variables.
- **Key Press Optimization**: The compiler optimizes the number of key presses required to enter addresses using the `npress` array.
- **Error Handling**: The compiler provides detailed error messages and warnings to help diagnose issues during the compilation process.

### Code Structure

## `compiler.py`

### Imports and Initial Setup
- **Imports**: The script imports necessary modules and functions from `libcompiler`.
- **Change Directory**: Changes the working directory to the script's location.
- **Append Path**: Adds the parent directory to the system path.

### ROM and Disassembly Loading
- **get_rom('rom.bin')**: Loads the ROM binary.
- **get_disassembly('disas.txt')**: Loads the disassembly file.
- **get_commands('gadgets')**: Loads the gadget commands.
- **read_rename_list('labels')**: Reads the rename list.
- **read_rename_list('../labels_sfr')**: Reads another rename list.

### Font and Npress Array Setup
- **FONT**: Defines a font array from a multi-line string.
- **set_font(FONT)**: Sets the font using the defined array.
- **npress**: Defines the npress array for key presses.
- **set_npress_array(npress)**: Sets the npress array.

### Symbol Representation
- **get_binary(filename)**: Reads a binary file and returns its content.
- **get_symbol**: Imports `get_char_table` and defines symbols using the font.
- **set_symbolrepr(symbols[:])**: Sets the symbol representation.

### Argument Parsing
- **argparse.ArgumentParser()**: Sets up argument parsing for command-line options.
- **args**: Parses the command-line arguments.

### Gadget Optimization and Address Printing
- **optimize_gadget**: Optimizes a gadget based on the provided arguments.
- **print_addresses**: Prints the optimized addresses.

### Program Processing
- **process_program(args, program, overflow_initial_sp)**: Processes the input program based on the arguments and initial stack pointer.

## `get_char_table.py`

### Imports and ROM Loading
- **Imports**: Imports necessary modules.
- **LOOKUP**: Defines a lookup table for character to string conversion.
- **ROMWINDOW**: Sets the ROM window size.

### Character Fetching
- **fetch(x)**: Fetches a 16-bit value from the ROM.
- **f(x)**: Converts a character to a string using the lookup table.

### Main Functionality
- **Main Block**: Iterates over the lookup table and prints the character and string conversion.

## `libcompiler.py`

### Imports and Constants
- **Imports**: Imports necessary modules and functions.
- **max_call_adr**: Sets the maximum call address.

### Font and Symbol Representation
- **set_font(font_)**: Sets the font and creates an association dictionary.
- **from_font(st)**: Converts a string to font indices.
- **to_font(charcodes)**: Converts font indices to a string.
- **set_npress_array(npress_)**: Sets the npress array.
- **set_symbolrepr(symbolrepr_)**: Sets the symbol representation.

### Key and Address Functions
- **byte_to_key(byte)**: Converts a byte to a key representation.
- **get_npress(charcodes)**: Calculates the number of key presses for a sequence of characters.
- **get_npress_adr(adrs)**: Calculates the number of key presses for a sequence of addresses.
- **optimize_adr_for_npress(adr)**: Optimizes an address for minimal key presses.
- **optimize_sum_for_npress(total)**: Optimizes a sum for minimal key presses.

### Utility Functions
- **note(st)**: Prints a note to stderr.
- **to_lowercase(s)**: Converts a string to lowercase.
- **canonicalize(st)**: Canonicalizes a string.
- **del_inline_comment(line)**: Removes inline comments from a line.
- **add_command(command_dict, address, command, tags, debug_info='')**: Adds a command to the command dictionary.

### Command and Label Processing
- **get_commands(filename)**: Reads a list of gadget names from a file.
- **get_disassembly(filename)**: Parses a disassembly file.
- **read_rename_list(filename)**: Parses a rename list.
- **sizeof_register(reg_name)**: Returns the size of a register.

### Program Processing
- **process(line)**: Processes a single line of the program.
- **finish_processing()**: Finishes processing deferred commands.
- **process_program(args, program, overflow_initial_sp)**: Processes the entire program.

### ROM and Address Functions
- **get_rom(x)**: Loads the ROM binary.
- **find_equivalent_addresses(rom: bytes, q: set)**: Finds equivalent addresses in the ROM.
- **optimize_gadget_f(rom: bytes, gadget: bytes)**: Optimizes a gadget in the ROM.
- **optimize_gadget(gadget: bytes)**: Optimizes a gadget.
- **print_addresses(adrs, n_preview: int)**: Prints the addresses of optimized gadgets.

######                                                                                                                      Written by hieuxyz