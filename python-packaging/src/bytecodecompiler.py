# Copyright 2022 Gregory Szorc.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

# This script is executed to compile Python bytecode by the repackager.
#
# When invoked, we start a server that listens for commands. We then
# react to those commands and send results to the caller.

import dis
import importlib._bootstrap_external
import importlib.util
import marshal
import os
import re
import sys
import types


RE_CODING = re.compile(b"^[ \t\f]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)")


# Marshal format 4 is used by Python 3.4 to 3.13; Python 3.14 bumped the
# version to 5. Update this check when a new marshal format is added.
if marshal.version not in (4, 5):
    raise Exception("unexpected marshal version: %d" % marshal.version)

stdin = sys.__stdin__.buffer
stdout = sys.__stdout__.buffer


def decode_source(source):
    """Decodes Python source code bytes into a string."""
    # Default source encoding is UTF-8. But per PEP 263, the first or second
    # line of source can match a regular expression to define a custom
    # encoding. We need to detect custom encodings and use it to decode
    # the passed bytes to str.
    encoding = "utf-8"

    for line in source.splitlines()[0:2]:
        m = RE_CODING.match(line)
        if m:
            encoding = m.group(1).decode("ascii")
            break

    # Someone has set us up the BOM! According to PEP 263 the file should
    # be interpreted as UTF-8.
    if source.startswith(b"\xef\xbb\xbf"):
        encoding = "utf-8"
        source = source[3:]

    return source.decode(encoding)


def reads_dunder_file(code):
    """Returns whether a code object or its nested code objects read a name or
    attribute called ``__file__``.

    String constants, comments and assignments to ``__file__`` are ignored.
    """
    for instruction in dis.get_instructions(code):
        if (
            instruction.opcode in dis.hasname
            and instruction.argval == "__file__"
            and not instruction.opname.startswith(("STORE_", "DELETE_"))
        ):
            return True

    return any(
        reads_dunder_file(const)
        for const in code.co_consts
        if isinstance(const, types.CodeType)
    )


def write_result(data):
    """Writes a successful result to the caller."""
    stdout.write(b"0\n")
    stdout.write(b"%d\n" % len(data))
    stdout.write(data)


def write_error(e):
    """Writes an error to the caller."""
    stdout.write(b"1\n")
    error_bytes = str(e).encode("utf-8", errors="replace")
    stdout.write(b"%d\n" % len(error_bytes))
    stdout.write(error_bytes)


while True:
    command = stdin.readline().rstrip()

    if command == b"exit":
        sys.exit(0)
    elif command == b"magic_number":
        stdout.write(importlib._bootstrap_external.MAGIC_NUMBER)
        stdout.flush()
    elif command == b"compile":
        try:
            name_len = stdin.readline().rstrip()
            source_len = stdin.readline().rstrip()
            optimize_level = stdin.readline().rstrip()
            output_mode = stdin.readline().rstrip()

            name_len = int(name_len)
            source_len = int(source_len)
            optimize_level = int(optimize_level)

            name = stdin.read(name_len)
            source = stdin.read(source_len)

            name = os.fsdecode(name)

            source_bytes = source
            source = decode_source(source)

            code = compile(source, name, "exec", optimize=optimize_level)
            bytecode = marshal.dumps(code)

            if output_mode == b"bytecode":
                out = bytecode
            elif output_mode == b"pyc-checked-hash":
                source_hash = importlib.util.source_hash(source_bytes)
                out = importlib._bootstrap_external._code_to_hash_pyc(
                    code,
                    source_hash,
                    checked=True,
                )
            elif output_mode == b"pyc-unchecked-hash":
                source_hash = importlib.util.source_hash(source_bytes)
                out = importlib._bootstrap_external._code_to_hash_pyc(
                    code,
                    source_hash,
                    checked=False,
                )
            else:
                raise Exception("unknown output mode: %s" % output_mode)

            write_result(out)
        except Exception as e:
            write_error(e)

        stdout.flush()
    elif command == b"reads_dunder_file":
        try:
            name_len = int(stdin.readline().rstrip())
            source_len = int(stdin.readline().rstrip())

            name = os.fsdecode(stdin.read(name_len))
            source = decode_source(stdin.read(source_len))

            code = compile(source, name, "exec", dont_inherit=True)
            write_result(b"1" if reads_dunder_file(code) else b"0")
        except Exception as e:
            write_error(e)

        stdout.flush()
    else:
        raise Exception("invalid command: %s" % command)
