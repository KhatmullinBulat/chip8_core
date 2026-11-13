struct Emulator {
    memory: [u8; 4096],      // оперативная память
    v: [u8; 16],             // регистры общего назначения
    i: u16,                  // индексный регистр
    pc: u16,                 // счетчик команд (0x200)
    stack: [u16; 16],        // стек вызовов
    sp: u8,                  // указатель стека
    dt: u8,                  // таймер задержки
    st: u8,                  // таймер звука
    screen: [bool; 64 * 32], // видеобуфер
    keypad: [bool; 16],      // состояние 16 клавиш
}

const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

// адрес, с которого начинается программа (512)
const START_ADDR: u16 = 0x200;
const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

impl Emulator {
    pub fn new() -> Self {
        let mut em = Self {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: START_ADDR,
            stack: [0; 16],
            sp: 0,
            dt: 0,
            st: 0,
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            keypad: [false; 16],
        };

        // копирование шрифтов в начало памяти (0x000..0x050)
        em.memory[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        return em;
    }

    // загрузка файлов игры в память начиная с 0x200
    pub fn load_rom(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.memory[start..end].copy_from_slice(data);
    }

    pub fn tick(&mut self) {
        // FETCH: берем 2 байта по адресу pc и склеиваем в u16 (Big-Endian)
        let op = self.fetch();

        // DECODE & EXECUTE
        self.execute(op);
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.memory[self.pc as usize] as u16;
        let lower_byte = self.memory[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;

        return op;
    }

    fn execute(&mut self, op: u16) {
        // разбивка 16 битного опкода на 4 полубайта
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // 0000 - NOP (пустая операция)
            (0, 0, 0, 0) => return,

            // 00E0 - CLS: очистить экран
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }

            // 1NNN - JMP: безусловный переход на адрес NNN
            (1, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.pc = nnn;
            }

            // 6XNN - LD Vx, byte: записать число NN в регистр VX
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                self.v[x] = nn;
            }

            // 7XNN - ADD Vx, byte: прибавить NN к регистру VX
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                self.v[x] = self.v[x].wrapping_add(nn);
            }

            // ANNN - LD I, addr: записать адрес NNN в индексный регистр I
            (0xA, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.i = nnn;
            }

            // DXYN - DRW Vx, Vy, nibble: отрисовка спрайта
            (0xD, _, _, _) => {
                todo!()
            }

            _ => unimplemented!("Неизвестный опкод: {:#06x}", op),
        }
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            if self.st == 1 {
                // todo
            }
            self.st -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_opcodes() {
        let mut em = Emulator::new();
        // Программа: 610A (V1 = 10), 7105 (V1 +=5), 1200 (обратно на 0x200)
        let rom = [0x61, 0x0A, 0x71, 0x05, 0x12, 0x00];
        em.load_rom(&rom);

        em.tick();
        assert_eq!(em.v[1], 10);

        em.tick();
        assert_eq!(em.v[1], 15);

        em.tick();
        assert_eq!(em.pc, 0x200)
    }
}
