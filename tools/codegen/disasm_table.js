const ARM_TABLE = [
    ["____00__________________________", "DataProcessing",               "Data Processing / PSR Transfer"],
    ["____000000______________1001____", "Multiply",                     "Multiply"],
    ["____00001_______________1001____", "MultiplyLong",                "Multiply Long"],
    ["____00010_00________00001001____", "SingleDataSwap",              "Single Data Swap"],
    ["____000100101111111111110001____", "BranchAndExchange",           "Branch and Exchange"],
    ["____000__0__________00001__1____", "HalfwordDataTransfer",        "Halfword Data Transfer (register offset)"],
    ["____000__1______________1__1____", "HalfwordDataTransfer",        "Halfword Data Transfer (immediate offset)"],
    ["____01__________________________", "SingleDataTransfer",          "Single Data Transfer"],
    ["____011____________________1____", "Undefined",                   "Undefined"],
    ["____100_________________________", "BlockDataTransfer",           "Block Data Transfer"],
    ["____101_________________________", "Branch",                      "Branch"],
    ["____110_________________________", "CoprocessorDataTransfer",     "Coprocessor Data Transfer"],
    ["____1110___________________0____", "CoprocessorDataOperation",    "Coprocessor Data Operation"],
    ["____1110___________________1____", "CoprocessorRegisterTransfer", "Coprocessor Register Transfer"],
    ["____1111________________________", "SoftwareInterrupt",           "Software Interrupt"],
];

const THUMB_TABLE = [
    ["000_____________", "MoveShiftedRegister",         "Move Shifted Register"],
    ["00011___________", "AddSubtract",                 "Add / Subtract"],
    ["001_____________", "MoveCompareAddSubtractImm",   "Move/ Compare/ Add/ Subtract Immediate"],
    ["010000__________", "ALUOperations",               "ALU Operations"],
    ["010001__________", "HiRegisterOperations",        "Hi Register Operations / Branch Exchange"],
    ["01001___________", "PCRelativeLoad",              "PC-relative Load"],
    ["0101__0_________", "LoadStoreWithRegisterOffset", "Load/Store with register offset"],
    ["0101__1_________", "LoadStoreSignHalfwordByte",   "Load/Store Sign-Extended Byte/Halfword"],
    ["911_____________", "LoadStoreWithImmOffset",      "Load/Store with Immediate Offset"],
    ["1000____________", "LoadStoreHalfword",           "Load/Store Halfword"],
    ["1001____________", "SPRelativeLoadStore",         "SP-relative Load/Store"],
    ["1010____________", "LoadAddress",                 "Load Address"],
    ["10110000________", "AddOffsetToStackPointer",     "Add Offset to Stack Pointer"],
    ["1011_10_________", "PushPopRegisters",            "Push/Pop Registers"],
    ["1100____________", "MultipleLoadStore",           "Multiple Load/Store"],
    ["1101____________", "ConditionalBranch",           "Conditional Branch"],
    ["11011111________", "SoftwareInterrupt",           "Software Interrupt"],
    ["11100___________", "UnconditionalBranch",         "Unconditional Branch"],
    ["1111____________", "LongBranchWithLink",          "Long Branch with Link"],
];


function createDisassemblyTable(tableName, opcodeBits, stringTable, enumName) {
    const uniqueEnumVariants = new Set();
    const tableString = stringTable.map(([bitString, enumVariant, description]) => {
        const significantSelectMask = createSignificantBitsSelectMask(bitString);
        const significantMask = createSignificantBitsMask(bitString);
        const significantBitsCount = countSignificantBits(bitString);
        return { significantMask, significantSelectMask, significantBitsCount, enumVariant, description };
    }).sort((a, b) => {
        return b.significantBitsCount - a.significantBitsCount;
    }).reduce((dest, instrType) => {
        uniqueEnumVariants.add(instrType.enumVariant);
        const selectMaskHex = toHex(instrType.significantSelectMask, opcodeBits); 
        const diffMaskHex = toHex(instrType.significantMask, opcodeBits); 
        dest += `\t(${selectMaskHex}, ${diffMaskHex}, ${enumName}::${instrType.enumVariant}), // ${instrType.description}\n`;
        return dest;
    }, "");

    let enumVariantsString = "";
    uniqueEnumVariants.forEach(variant => {
        enumVariantsString += `\t${variant},\n`;
    });

    const tableCode = `const ${tableName}: [(u32, u32, ${enumName}); ${stringTable.length}] = [\n${tableString}\n];`;
    const enumCode = `pub enum ${enumName} {\n${enumVariantsString}}`;

    return [tableCode, enumCode];
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

function fixTabs(s) {
    return s.replace(/\t/g, '    ');
}

function main() {
    const [armTable, armEnum] = createDisassemblyTable("ARM_OPCODE_TABLE", 32, ARM_TABLE, "ARMInstrType");
    console.log("// ARM");
    console.log(fixTabs(armTable));
    console.log();
    console.log(fixTabs(armEnum));

    console.log("\n");

    const [thumbTable, thumbEnum] = createDisassemblyTable("THUMB_OPCODE_TABLE", 16, THUMB_TABLE, "THUMBInstrType");
    console.log("// THUMB");
    console.log(fixTabs(thumbTable));
    console.log();
    console.log(fixTabs(thumbEnum));
}
main();
