const ARM_TABLE = [
    ["____00__________________________", "ARMInstrType::DataProcessing",                "Data Processing / PSR Transfer"],
    ["____000000______________1001____", "ARMInstrType::Multiply",                      "Multiply"],
    ["____00001_______________1001____", "ARMInstrType::MultiplyLong",                 "Multiply Long"],
    ["____00010_00________00001001____", "ARMInstrType::SingleDataSwap",                "Single Data Swap"],
    ["____000100101111111111110001____", "ARMInstrType::BranchAndExchange",             "Branch and Exchange"],
    ["____000__0__________00001__1____", "ARMInstrType::HalfwordDataTransfer",          "Halfword Data Transfer (register offset)"],
    ["____000__1______________1__1____", "ARMInstrType::HalfwordDataTransfer",          "Halfword Data Transfer (immediate offset)"],
    ["____01__________________________", "ARMInstrType::SingleDataTransfer",            "Single Data Transfer"],
    ["____011____________________1____", "ARMInstrType::Undefined",                     "Undefined"],
    ["____100_________________________", "ARMInstrType::BlockDataTransfer",             "Block Data Transfer"],
    ["____101_________________________", "ARMInstrType::Branch",                        "Branch"],
    ["____110_________________________", "ARMInstrType::CoprocessorDataTransfer",       "Coprocessor Data Transfer"],
    ["____1110___________________0____", "ARMInstrType::CoprocessorDataOperation",      "Coprocessor Data Operation"],
    ["____1110___________________1____", "ARMInstrType::CoprocessorRegisterTransfer",   "Coprocessor Register Transfer"],
    ["____1111________________________", "ARMInstrType::SoftwareInterrupt",             "Software Interrupt"],
];

const THUMB_TABLE = [
    ["000_____________", "THUMBInstrType::MoveShiftedRegister",         "Move Shifted Register"],
    ["00011___________", "THUMBInstrType::AddSubtract",                 "Add / Subtract"],
    ["001_____________", "THUMBInstrType::MoveCompareAddSubtractImm",   "Move/ Compare/ Add/ Subtract Immediate"],
    ["010000__________", "THUMBInstrType::ALUOperations",               "ALU Operations"],
    ["010001__________", "THUMBInstrType::HiRegisterOperations",        "Hi Register Operations / Branch Exchange"],
    ["01001___________", "THUMBInstrType::PCRelativeLoad",              "PC-relative Load"],
    ["0101__0_________", "THUMBInstrType::LoadStoreWithRegisterOffset", "Load/Store with register offset"],
    ["0101__1_________", "THUMBInstrType::LoadStoreSignHalfwordByte",   "Load/Store Sign-Extended Byte/Halfword"],
    ["911_____________", "THUMBInstrType::LoadStoreWithImmOffset",      "Load/Store with Immediate Offset"],
    ["1000____________", "THUMBInstrType::LoadStoreHalfword",           "Load/Store Halfword"],
    ["1001____________", "THUMBInstrType::SPRelativeLoadStore",         "SP-relative Load/Store"],
    ["1010____________", "THUMBInstrType::LoadAddress",                 "Load Address"],
    ["10110000________", "THUMBInstrType::AddOffsetToStackPointer",     "Add Offset to Stack Pointer"],
    ["1011_10_________", "THUMBInstrType::PushPopRegisters",            "Push/Pop Registers"],
    ["1100____________", "THUMBInstrType::MultipleLoadStore",           "Multiple Load/Store"],
    ["1101____________", "THUMBInstrType::ConditionalBranch",           "Conditional Branch"],
    ["11011111________", "THUMBInstrType::SoftwareInterrupt",           "Software Interrupt"],
];


function createDisassemblyTable(tableName, opcodeBits, stringTable) {
    const tableString = stringTable.map(([bitString, enumName, description]) => {
        const significantSelectMask = createSignificantBitsSelectMask(bitString);
        const significantMask = createSignificantBitsMask(bitString);
        const significantBitsCount = countSignificantBits(bitString);
        return { significantMask, significantSelectMask, significantBitsCount, enumName, description };
    }).sort((a, b) => {
        return b.significantBitsCount - a.significantBitsCount;
    }).reduce((dest, instrType) => {
        dest += `    (${toHex(instrType.significantSelectMask, opcodeBits)}, ${toHex(instrType.significantMask, opcodeBits)}), // ${instrType.description}\n`;
        return dest;
    }, "");

    return `const ${tableName}: [(u32, u32, ARMInstrType); ${stringTable.length}] = [\n${tableString}\n];`;
}

function toHex(value, bits) {
    const len = Math.ceil(bits / 4);
    let hex = value.toString(16);
    while (hex.length < len) {
        hex = '0' + hex;
    }
    return '0x' + hex;
}

/// Takes a bit string with insignificant bits replaced with '_' and
/// returns a bit mask with only the significant '1' bits set that
/// can be diffed (xored) with the output of `createSignificantBitsSelectMask`.
function createSignificantBitsMask(bitString) {
    let mask = 0x0;
    let bitMax = bitString.length - 1;
    for (let stringOffset = 0; stringOffset < bitString.length; stringOffset++) {
        if (bitString[stringOffset] == '1') {
            const bitOffset = bitMax - stringOffset;
            mask |= 1 << bitOffset;
        }
    }
    return mask;
}

/// Takes a bit string with insignificant bits replaced with '_' and
/// returns a bit mask that can be used to select only the significant
/// bits from an integer.
function createSignificantBitsSelectMask(bitString) {
    let signigicantMask = 0x0;
    let bitMax = bitString.length - 1;
    for (let stringOffset = 0; stringOffset < bitString.length; stringOffset++) {
        if (bitString[stringOffset] != '_') {
            const bitOffset = bitMax - stringOffset;
            signigicantMask |= 1 << bitOffset;
        }
    }
    return signigicantMask;
}

function countSignificantBits(bitString) {
    let count = 0;
    for (let stringOffset = 0; stringOffset < bitString.length; stringOffset++) {
        if (bitString[stringOffset] != '_') {
            count += 1;
        }
    }
    return count;
}

function main() {
    const armTable = createDisassemblyTable("ARM_OPCODE_TABLE", 32, ARM_TABLE);
    console.log("// ARM TABLE");
    console.log(armTable);

    console.log("\n");

    const thumbTable = createDisassemblyTable("THUMB_OPCODE_TABLE", 16, THUMB_TABLE);
    console.log("// THUMB TABLE");
    console.log(thumbTable);
}
main();
