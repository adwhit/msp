
use ncurses::*;
use mem;
use cpu;
use std;



static RAMHEIGHT : i32 = 50;
static RAMWIDTH : i32 = 49;
static RAMX : i32 = 1;
static RAMY : i32 = 1;
static REGHEIGHT : i32 = 7;
static REGWIDTH : i32 = 37;
static REGX : i32 = 52;
static REGY : i32 = 1;
static ASMHEIGHT : i32 = 50;
static ASMWIDTH : i32 = 30;
static ASMX : i32 = 52;
static ASMY : i32 = 1;
static DBGHEIGHT : i32 = 50;
static DBGWIDTH : i32 = 30;
static DBGX : i32 = 52;
static DBGY : i32 = 30;

pub struct Gui {
    ramwin : WINDOW,
    regwin : WINDOW,
    asmwin : WINDOW,
    dbgwin : WINDOW,
}

impl Gui {

    pub fn init() -> Gui {
        initscr();
        clear();
        raw();
        noecho();
        start_color();
        init_pair(1, COLOR_RED, COLOR_WHITE);

        let ramwin = newwin(RAMHEIGHT, RAMWIDTH, RAMY, RAMX);
        let regwin = newwin(REGHEIGHT, REGWIDTH, REGY, REGX);
        let asmwin = newwin(ASMHEIGHT, ASMWIDTH, ASMY, ASMX);
        let dbgwin = newwin(DBGHEIGHT, DBGWIDTH, DBGY, DBGX);

        box_(ramwin, 0, 0);
        box_(regwin, 0, 0);
        box_(asmwin, 0, 0);
        box_(dbgwin, 0, 0);

        mvwprintw(ramwin,0, 10, "RAM");
        mvwprintw(regwin,0, 10, "Registers");

        refresh();

        Gui {
            ramwin: ramwin,
            regwin: regwin,
            asmwin: asmwin,
            dbgwin: dbgwin
        }
    }

    pub fn draw_ram(&self, r: mem::Ram, pc: u16) {
        let mut rowct = 1;
        for row in std::iter::range(0, r.arr.len()/16) {
            let mut nonzero = false;
            for col in range(0, 16u) {
                if r.arr[row * 16 + col] != 0 { nonzero = true } 
            }
            if !nonzero { continue };
            wmove(self.ramwin, rowct, 1); rowct += 1;
            wprintw(self.ramwin, format!("{:04x}:  ", row * 16));
            for col in range(0u, 16u) { 
                if col % 2 == 0 {
                    // take two at once
                    if row * 16 + col == pc as uint  {
                        // print in colour
                        wattron(self.ramwin, COLOR_PAIR(1));
                        wprintw(self.ramwin, format!("{:02x}{:02x} ", r.arr[row * 16 + col], r.arr[row * 16 + col + 1]));
                        wattroff(self.ramwin, COLOR_PAIR(1));
                    } else {
                        // normal print
                        wprintw(self.ramwin, format!("{:02x}{:02x} ", r.arr[row * 16 + col], r.arr[row * 16 + col + 1]));
                    }
                }
            }
        }
        wrefresh(self.ramwin);
    }

    pub fn draw_regs(&self, r: mem::Regs, inst: &str) {
        mvwprintw(self.regwin,1,1, format!("PC  {:04x} SP  {:04x} SR  {:04x} CG  {:04x}",
            r.arr[0], r.arr[1], r.arr[2], r.arr[3]));
        mvwprintw(self.regwin,2,1, format!("R04 {:04x} R05 {:04x} R06 {:04x} R07 {:04x}",
            r.arr[4], r.arr[5], r.arr[6], r.arr[7]));
        mvwprintw(self.regwin,3,1, format!("R08 {:04x} R09 {:04x} R10 {:04x} R11 {:04x}",
            r.arr[8], r.arr[9], r.arr[10], r.arr[11]));
        mvwprintw(self.regwin,4,1, format!("R12 {:04x} R13 {:04x} R14 {:04x} R15 {:04x}",
            r.arr[12], r.arr[13], r.arr[14], r.arr[15]));
        mvwprintw(self.regwin,5,14, inst);
        wrefresh(self.regwin);
    }



    pub fn render(&self, cpu: cpu::Cpu) {
        self.draw_ram(cpu.ram, cpu.regs.arr[0]);
        self.draw_regs(cpu.regs, cpu.inst.to_string());
        mvprintw(LINES - 1, 0, "S to advance, Q to exit");
        move(10, 10);
        refresh();
    }

}
