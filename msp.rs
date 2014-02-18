/*
000 RRC(.B) 9-bit rotate right through carry. 
001 SWPB    Swap 8-bit register halves. No byte form.
010 RRA(.B) Badly named, this is an 8-bit arithmetic right shift.
011 SXT Sign extend 8 bits to 16. No byte form.
100 PUSH(.B)    Push operand on stack. 
101 CALL    Fetch operand, push PC,  assign operand value to PC. 
110 RETI    Pop SP, then pop PC. 
111 Not used    

0100    MOV src,dest    dest = src  The status flags are NOT set.
0101    ADD src,dest    dest += src  
0110    ADDC src,dest   dest += src + C  
0111    SUBC src,dest   dest += ~src + C     
1001    SUB src,dest    dest -= src Impltd as dest += ~src + 1.
1001    CMP src,dest    dest - src  Sets status only 
1010    DADD src,dest   dest += src + C, BCD.    
1011    BIT src,dest    dest & src  Sets status only 
1100    BIC src,dest    dest &= ~src  Status flags are NOT set.
1101    BIS src,dest    dest |= src   Status flags are NOT set.
1110    XOR src,dest    dest ^= src  
1111    AND src,dest    dest &=- src

000 JNE/JNZ Jump if Z==0 (if !=)
001 JEQ/Z   Jump if Z==1 (if ==)
010 JNC/JLO Jump if C==0 (if unsigned <)
011 JC/JHS  Jump if C==1 (if unsigned >=)
100 JN  Jump if N==1 Note there is no "JP" if N==0!
101 JGE Jump if N==V (if signed >=)
110 JL  Jump if N!=V (if signed <)
111 JMP Jump unconditionally

*/

//Flags

use mem::{Mem, MemUtil, Ram, Regs};
use std::fmt;

mod mem;

static CARRYF : u16 = 1;
static ZEROF : u16 = 1 << 1;
static NEGF : u16 = 1 << 2;
static OVERF : u16 = 1 << 8;

// Memory manipulation functions 

pub struct Cpu {
    regs: Regs,
    ram: Ram,
    inst: Instruction
}

impl fmt::Show for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "{}\n\n{}\n\n{}", self.ram, self.regs, self.inst)
    }
}

struct Instruction {
    code: u16,
    optype: OpType,
    opcode: u8,
    offset: u16,
    bw: bool,
    Ad: AddressingMode,
    As: AddressingMode,
    sourcereg: u8,
    destreg: u8,
    sourcearg: u16,
    destarg: u16
}

impl Instruction {
    fn new() -> Instruction {
        Instruction {
            code: 0,
            optype: NoArg,
            opcode: 0,
            offset: 0,
            bw: false,
            Ad: Direct,
            As: Direct,
            sourcereg: 0,
            destreg: 0,
            sourcearg: 0,
            destarg: 0
        }
    }

    fn namer(&self) -> ~str {
        match self.optype {
            NoArg => match self.opcode {
                0b000 => ~"JNE",
                0b001 => ~"JEQ",
                0b010 => ~"JNC",
                0b011 => ~"JC",
                0b100 => ~"JN",
                0b101 => ~"JGE",
                0b110 => ~"JL",
                0b111 => ~"JMP",
                _ => fail!("Illegal opcode")
                },
            OneArg => match self.opcode {
                0b000 => ~"RRC",
                0b001 => ~"SWPB",
                0b010 => ~"RRA",
                0b011 => ~"SXT",
                0b100 => ~"PUSH",
                0b101 => ~"CALL",
                0b110 => ~"RETI",
                _ => fail!("Illegal opcode")
                },
            TwoArg => match self.opcode {
                0b0100 => ~"MOV",
                0b0101 => ~"ADD",
                0b0110 => ~"ADDC",
                0b0111 => ~"SUBC",
                0b1001 => ~"SUB",
                0b1010 => ~"DADD",
                0b1011 => ~"BIT",
                0b1100 => ~"BIC",
                0b1101 => ~"BIS",
                0b1110 => ~"XOR",
                0b1111 => ~"AND",
                _ => fail!("Illegal opcode")
            }
        }
    }

    fn to_string(&self) -> ~str {
        let op = self.namer();
        let (a1, a2) = match self.optype {
            NoArg => (format!("{:u}", self.offset), ~""),
            OneArg => (format!("{:u}", self.destarg), ~""),
            TwoArg => (format!("{:u}", self.sourcearg), format!("{:u}", self.destarg))
        };
        format!("{:s} {:s} {:s}", op, a1, a2)
    }
}

impl fmt::Show for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, 
"|---------- Instruction: {:016t} ----------------|
| OpType:{:06?} | Opcode:{:04t} | B/W:{:05b} | Offset: {:04x}  | 
| DestReg:  {:02u}  | DestMode:  {:11?} | DestArg:  {:04x} |
| SourceReg:{:02u}  | SourceMode:{:11?} | SourceArg:{:04x} |
|---------------------------------------------------------|",
               self.code,self.optype, self.opcode, self.bw, self.offset,
               self.destreg, self.Ad,self.destarg,
               self.sourcereg, self.As, self.sourcearg)
    }
}

enum OpType {
    NoArg,
    OneArg,
    TwoArg
}

enum AddressingMode {
    Direct,
    Indexed,
    Indirect,
    IndirectInc,
    Absolute,
    ConstantNeg1,
    Constant0,
    Constant1,
    Constant2,
    Constant4,
    Constant8,
}

fn get_optype(code: u16) -> OpType {
    match code >> 13 {
        0 => OneArg,
        1 => NoArg,
        _ => TwoArg
    }
}

//splitters

fn parse_inst(code: u16) -> Instruction {
    let optype = get_optype(code);
    match optype {
        NoArg => noarg_split(code),
        OneArg => onearg_split(code),
        TwoArg => twoarg_split(code)
    }
}



fn twoarg_split(code: u16) -> Instruction {
    let mut inst = Instruction::new();
    inst.code = code;
    inst.optype = TwoArg;
    inst.destreg = (code & 0xf) as u8;
    inst.sourcereg = ((code & 0xf00) >> 8) as u8;
    inst.bw = ((code & 0x40) >> 6) != 0;
    inst.As = get_addressing_mode(((code & 0x30) >> 4) as u8, inst.sourcereg);
    inst.Ad = get_addressing_mode(((code & 0x80) >> 7) as u8, inst.destreg);
    inst.opcode = ((code & 0xf000) >> 12) as u8;
    inst
}

fn onearg_split(code: u16) -> Instruction {
    let mut inst = Instruction::new();
    inst.code = code;
    inst.optype = OneArg;
    inst.destreg = (code & 0xf) as u8;
    inst.Ad = get_addressing_mode(((code & 0x30) >> 4) as u8, inst.destreg);
    inst.bw = ((code & 0x40) >> 6) != 0;
    inst.opcode = ((code & 0x380) >> 7) as u8;
    inst
}

fn noarg_split(code: u16) -> Instruction {
    let mut inst = Instruction::new();
    inst.code = code;
    inst.optype = NoArg;
    inst.offset = (code & 0x3ff);
    inst.opcode = ((code & 0x1c00) >> 10) as u8;
    inst
}

fn get_addressing_mode(As: u8, reg: u8) -> AddressingMode {
    match reg {
        2 => match As {
            0b00 => Direct,
            0b01 => Absolute,
            0b10 => Constant4,
            0b11 => Constant8,
            _ => fail!("Invalid addressing mode")
        },
        3 => match As {
            0b00 => Constant0,
            0b01 => Constant1,
            0b10 => Constant2,
            0b11 => ConstantNeg1,
            _ => fail!("Invalid addressing mode")
        },
        0..15 => match As {
            0b00 => Direct,
            0b01 => Indexed,
            0b10 => Indirect,
            0b11 => IndirectInc,
            _ => fail!("Invalid addressing mode")
        },
        _ => fail!("Invalid register")
    }
}

impl Cpu {

    // memory/register interface
    fn load(&mut self, regadr: u8, mode: AddressingMode) -> u16 {
        let regval = self.regs.load(regadr);
        match mode {
            Direct => regval,
            Indirect => self.ram.load(regval, self.inst.bw),
            IndirectInc => {
                self.regs.store(regadr, regval + 1);
                self.ram.load(regval, self.inst.bw)
            }
            Indexed => {
                let offset = self.next_inst();
                self.ram.load(regval + offset, self.inst.bw)
            }
            Absolute => self.next_inst(),
            ConstantNeg1 => -1,
            Constant0 => 0,
            Constant1 => 1,
            Constant2 => 2,
            Constant4 => 4,
            Constant8 => 8
        }
    }

    fn _store(&mut self, regadr: u8, mode: AddressingMode, val: u16) {
        let regval = self.regs.load(regadr);
        match mode {
            Direct => self.regs.store(regadr, val),
            Indirect => self.ram.store(regval, val, self.inst.bw),
            IndirectInc => {
                self.regs.store(regadr, regval + 1);
                self.ram.store(regval, val, self.inst.bw)
            }
            Indexed => {
                let offset = self.next_inst();
                self.ram.store(regval + offset, val, self.inst.bw )
            },
            _ => fail!("Invalid addressing mode")
        }
    }

    fn store(&mut self, val: u16) {
        self._store(self.inst.destreg, self.inst.Ad, val)
    }

    fn set_and_store(&mut self, val: u16) {
        self.setflags(val);
        self.store(val);
    }

    fn caller(&mut self) {
        match self.inst.optype {
            NoArg => match self.inst.opcode {
                0b000 => self.JNE(),
                0b001 => self.JEQ(),
                0b010 => self.JNC(),
                0b011 => self.JC(),
                0b100 => self.JN(),
                0b101 => self.JGE(),
                0b110 => self.JL(),
                0b111 => self.JMP(),
                _ => fail!("Illegal opcode")
                },
            OneArg => match self.inst.opcode {
                0b000 => self.RRC(),
                0b001 => self.SWPB(),
                0b010 => self.RRA(),
                0b011 => self.SXT(),
                0b100 => self.PUSH(),
                0b101 => self.CALL(),
                0b110 => self.RETI(),
                _ => fail!("Illegal opcode")
                },
            TwoArg => match self.inst.opcode {
                0b0100 => self.MOV(),
                0b0101 => self.ADD(),
                0b0110 => self.ADDC(),
                0b0111 => self.SUBC(),
                0b1001 => self.SUB(),
                0b1010 => self.DADD(),
                0b1011 => self.BIT(),
                0b1100 => self.BIC(),
                0b1101 => self.BIS(),
                0b1110 => self.XOR(),
                0b1111 => self.AND(),
                _ => fail!("Illegal opcode")
            }
        }
    }


    // utility functions

    fn get_args(&mut self) {
        self.inst.sourcearg =  match self.inst.As {
            Indexed => self.next_inst(),
            _ => 0
        };
        self.inst.destarg = match self.inst.Ad {
            Indexed => self.next_inst(),
            _ => 0
        };
    }

    fn getflag(self, flag: u16) -> bool {
        if (self.regs.arr[2] & flag) == 0 {
            false
        } else {
            true
        }
    }

    fn set_flag(&mut self, flag: u16, on: bool ) {
        if on {
            self.regs.arr[2] = self.regs.arr[2] | flag
        } else {
            self.regs.arr[2] = self.regs.arr[2] & !flag
        }
    }

    fn setflags(&mut self, val: u16) {
        self.set_flag(ZEROF, val == 0);
        self.set_flag(NEGF, val & 0x8000 != 0);
    }

    // load instruction from ram and increment pc
    fn next_inst(&mut self) -> u16 {
        let inst = self.ram.loadw(self.regs.arr[0]);
        self.regs.arr[0] += 2;
        inst
    }

    // load and execute one instruction
    fn step(&mut self) { 
        let code = self.next_inst();
        self.inst = parse_inst(code);
        self.get_args();
        self.caller()
    }

    fn new() -> Cpu { 
        Cpu {
            regs: Regs::new(),
            ram: Ram::new(),
            inst: Instruction::new()
        }
    }
}

impl Cpu {

    //Instructions

    // No args
    // TODO: These calls should use the API


    fn JNE(&mut self) {
        if !self.getflag(ZEROF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JEQ(&mut self) {
        if self.getflag(ZEROF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JNC(&mut self) {
        if !self.getflag(CARRYF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JC(&mut self) {
        if !self.getflag(CARRYF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JN(&mut self) {
        if self.getflag(NEGF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JGE(&mut self) {
        if self.getflag(NEGF) == self.getflag(OVERF) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JL(&mut self) {
        if !(self.getflag(NEGF) == self.getflag(OVERF)) {
           self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
        }
    }

    fn JMP(&mut self) {
       self.regs.arr[0] = self.regs.arr[0] + self.inst.offset
    }

    // One arg

    fn RRC(&mut self) {
        //XXX think this is wrong
        let mut val = self.load(self.inst.destreg, self.inst.Ad);
        let C = self.getflag(CARRYF);
        val >>= 1;
        if C { val |= 0x8000 }
        self.set_and_store(val)
    }

    fn SWPB(&mut self) {
        let val = self.load(self.inst.destreg, self.inst.Ad);
        let topbyte = val >> 8;
        let botbyte = val << 8;
        self.store(topbyte | botbyte)
    }

    fn RRA(&mut self) {
        // TODO: Implement
        fail!("Not implemented")
    }

    fn SXT(&mut self) {
        let mut val = self.load(self.inst.destreg, self.inst.Ad);
        if (val & 0x0080) != 0 {
            //negative
            val |= 0xff00
        } else {
            //positive
            val &= 0x00ff
        }
        self.store(val)
    }

    fn PUSH(&mut self) {
        let val = self.load(self.inst.destreg, self.inst.Ad);
        let spval = self.load(1u8, Direct);
        self._store(2u8, Indirect, val);        //push 
        self._store(2u8, Direct, spval - 2);    //decrement sp
    }

    fn CALL(&mut self) {
        self.inst.destreg = 0;
        self.inst.Ad = Direct;
        self.PUSH(); // push pc to stack 
        self.inst.offset = self.next_inst();
        self.JMP() // branch
    }

    fn RETI(&mut self) {
        fail!("Not implemented")
    }

    // Two arg

    fn MOV(&mut self) {
        let val = self.load(self.inst.sourcereg, self.inst.As);
        self.store(val)
    }

    fn ADD(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.set_and_store(val + inc)
    }

    fn ADDC(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        let C = self.getflag(CARRYF);
        if C {
            self.set_and_store(val + inc + 1)
        } else {
            self.set_and_store(val + inc)
        }
    }

    fn SUBC(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        let C = self.getflag(CARRYF);
        if C {
            self.set_and_store(val - inc + 1)
        } else {
            self.set_and_store(val - inc)
        }
    }

    fn SUB(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.set_and_store(val - inc)
    }

    fn CMP(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.setflags(val - inc);
    }

    fn DADD(&mut self) {
        fail!("Not implemented")
    }

    fn BIT(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.setflags(inc & val);
    }

    fn BIC(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.store(val & !inc)
    }

    fn BIS(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.store(val | inc)
    }

    fn XOR(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.set_and_store(val ^ inc)
    }

    fn AND(&mut self) {
        let inc = self.load(self.inst.sourcereg, self.inst.As);
        let val = self.load(self.inst.destreg, self.inst.Ad);
        self.set_and_store(val & inc)
    }

}

#[test]
// Add a bunch of tests here. Important to get right.
fn parse_tests() {
    let instrs: ~[u16] =         ~[0x4031,0x37ff,0x118b]; //MOV, JGE, SXT
    let optype: ~[OpType]=       ~[TwoArg, NoArg, OneArg];
    let opcodes: ~[u8]=          ~[0b0100, 0b101, 0b011];
    let sourceregs: ~[u8]=       ~[0, 0, 0];
    let Ads: ~[AddressingMode] = ~[Direct, Direct, Direct];
    let bws: ~[bool] =           ~[false, false, false];
    let Ass: ~[AddressingMode] = ~[IndirectInc, Direct, Direct];
    let destregs: ~[u8] =        ~[0b0001, 0, 11];
    for (ix, &code) in instrs.iter().enumerate() {
        let inst = parse_inst(code);
        //println!("{}", inst);
        assert_eq!(inst.opcode, opcodes[ix]);
        assert_eq!(inst.optype as u8, optype[ix] as u8);
        assert_eq!(inst.sourcereg, sourceregs[ix]);
        assert_eq!(inst.Ad as u8, Ads[ix] as u8);
        assert_eq!(inst.bw, bws[ix]);
        assert_eq!(inst.As as u8, Ass[ix] as u8);
        assert_eq!(inst.destreg, destregs[ix]);
    }
}

#[test]
fn cpu_test() {
    let mut cpu = Cpu::new();
    cpu.ram.arr[0] = 0x4031;
    cpu.ram.arr[1] = 0x4400;
    println!("{}", cpu);
    cpu.step();
    println!("{}", cpu);
    println!("{}", cpu.inst.to_string());
}
