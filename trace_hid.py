import pefile
import capstone
import sys

def analyze_dll(dll_path):
    print("=" * 60)
    print("ANALYZING:", dll_path)
    print("=" * 60)
    pe = pefile.PE(dll_path)
    image_base = pe.OPTIONAL_HEADER.ImageBase
    cs = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    cs.detail = True

    # Find exports and imports
    exports = {}
    if hasattr(pe, "DIRECTORY_ENTRY_EXPORT"):
        for exp in pe.DIRECTORY_ENTRY_EXPORT.symbols:
            name = exp.name.decode("utf-8") if exp.name else f"ord_{exp.ordinal}"
            exports[name] = exp.address

    print("Exports found:", len(exports))
    for target in ["hid_open", "hid_send_feature_report", "hid_get_feature_report", "hid_write"]:
        if target in exports:
            rva = exports[target]
            print(f"  {target} is at RVA: 0x{rva:X} (VA: 0x{image_base + rva:X})")

    # Let's search code section for calls/references to target RVAs or functions
    text_section = None
    for sec in pe.sections:
        name = sec.Name.decode("latin-1").strip("\x00")
        if name == ".text":
            text_section = sec
            break

    if not text_section:
        print("No .text section found!")
        return

    text_data = text_section.get_data()
    text_va = image_base + text_section.VirtualAddress

    for target in ["hid_open", "hid_send_feature_report", "hid_get_feature_report"]:
        if target not in exports:
            continue
        tgt_rva = exports[target]
        tgt_va = image_base + tgt_rva
        print(f"\nSearching callers of {target} (VA 0x{tgt_va:X})...")
        
        # Disassemble and find call instructions
        callers = []
        for insn in cs.disasm(text_data, text_va):
            if insn.mnemonic.startswith("call"):
                # Check target
                try:
                    op = insn.operands[0]
                    if op.type == capstone.x86.X86_OP_IMM:
                        if op.imm == tgt_va:
                            callers.append(insn.address)
                except:
                    pass
        print(f"Found {len(callers)} call sites for {target}:")
        for addr in callers[:10]:
            print(f"  Call site at 0x{addr:X}")
            # Disassemble around call site (30 instructions prior)
            offset = addr - text_va
            dis_start = max(0, offset - 120)
            dis_len = min(len(text_data) - dis_start, 150)
            sub_bytes = text_data[dis_start:dis_start+dis_len]
            print("  --- Disassembly leading up to call: ---")
            for sub_insn in cs.disasm(sub_bytes, text_va + dis_start):
                print(f"    0x{sub_insn.address:X}: {sub_insn.mnemonic} {sub_insn.op_str}")
                if sub_insn.address == addr:
                    break

analyze_dll("/home/aayam422h/Legion_Space/extracted/se/lgtplugins/SEBasicLighting/SEBasicLighting64.dll")
analyze_dll("/home/aayam422h/Legion_Space/extracted/se/plugins/SELuminousNB/SELuminousNB64.dll")

