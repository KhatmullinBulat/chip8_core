mod stack;

use stack::Stack;

#[cfg(feature = "SUPER-CHIP")]
const IS_SUPER_CHIP: bool = true;

#[cfg(not(feature = "SUPER-CHIP"))]
const IS_SUPER_CHIP: bool = false;

pub const SCREEN_WIDTH: usize = if IS_SUPER_CHIP { 128 } else { 64 };
pub const SCREEN_HEIGHT: usize = if IS_SUPER_CHIP { 64 } else { 32 };

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

pub struct Emulator {
    memory: [u8; 4096],                           // оперативная память
    v: [u8; 16],                                  // регистры общего назначения
    i: u16,                                       // индексный регистр
    pc: u16,                                      // счетчик команд (0x200)
    stack: Stack,                                 // стек вызовов
    dt: u8,                                       // таймер задержки
    st: u8,                                       // таймер звука
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT], // видеобуфер
    keypad: [bool; 16],                           // состояние 16 клавиш
}

impl Emulator {
    pub fn new() -> Self {
        let mut em = Self {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: START_ADDR,
            stack: Stack::new(),
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

            // 2NNN: вызов подпрограммы (Call)
            (2, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.stack.push(self.pc);
                self.pc = nnn;
            }

            // 00EE: возврат из подпрограммы (Return)
            (0, 0, 0xE, 0xE) => {
                self.pc = self.stack.pop();
            }

            // 3XNN: условный пропуск следующей команды
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                if self.v[x] == nn {
                    self.pc += 2;
                }
            }

            // 4XNN: условный пропуск следующей команды
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                if self.v[x] != nn {
                    self.pc += 2;
                }
            }

            // 5XY0: условный пропуск следующей команды
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }

            // 9XY0: условный пропуск следующей команды
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
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

            // 8XY0 - Set
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v[x] = self.v[y];
            }

            // 8XY1 - Binary OR
            (8, _, _, 1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v[x] = self.v[x] | self.v[y];
            }

            // 8XY2 - Binary AND
            (8, _, _, 2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v[x] = self.v[x] & self.v[y];
            }

            // 8XY3 - Logical XOR
            (8, _, _, 3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v[x] = ((self.v[x] != 0) ^ (self.v[y] != 0)) as u8;
            }

            // 8XY4 - Add: VX = VX + VY
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (sum, overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = sum;
                self.v[0xF] = if overflow { 1 } else { 0 };
            }

            // 8XY5 - Subtract: VX = VX - VY
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (diff, borrow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = diff;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }

            // 8XY7 - Subtract: VX = VY - VX
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (diff, borrow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = diff;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }

            // 8XY6 - Shift to the right (SHR)
            (8, _, _, 6) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                // original CHIP-8
                if !IS_SUPER_CHIP {
                    self.v[x] = self.v[y];
                }
                let lsb = self.v[x] & 0x1; // младший (самый правый) бит
                self.v[x] >>= 1; // сдвиг вправо на 1 бит
                self.v[0xF] = lsb;
            }

            // 8XYE - Shift to the right (SHL)
            (8, _, _, 0xE) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                // original CHIP-8
                if !IS_SUPER_CHIP {
                    self.v[x] = self.v[y];
                }
                let msb = (self.v[x] & 0x80) >> 7; // старший бит и сдвигаем его в младший разряд
                self.v[x] <<= 1; // сдвиг влево на 1 бит
                self.v[0xF] = msb;
            }

            // ANNN - LD I, addr: записать адрес NNN в индексный регистр I
            (0xA, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.i = nnn;
            }

            // BNNN and BXNN for SUPER_CHIP: прыжок со смещением
            (0xB, _, _, _) => {
                let nnn = op & 0x0FFF;
                if IS_SUPER_CHIP {
                    let x = digit2 as usize;
                    self.pc = nnn + self.v[x] as u16;
                } else {
                    self.pc = nnn + self.v[0] as u16;
                }
            }

            // CXNN - Random
            (0xC, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0x00FF) as u8;
                let random_byte: u8 = rand::random();
                self.v[x] = random_byte & nn;
            }

            // DXYN - DRW Vx, Vy, nibble: отрисовка спрайта
            (0xD, _, _, _) => {
                let n = (op & 0x000F) as u8;
                let x = digit2 as usize;
                let y = digit3 as usize;

                // начальные координаты заворачиваются по размеру экрана
                let start_x = self.v[x] % SCREEN_WIDTH as u8;
                let start_y = self.v[y] % SCREEN_HEIGHT as u8;

                // сброс флага коллизий перед отрисовкой
                self.v[0xF] = 0;

                // проходим по каждой строке спрайта
                for row in 0..n {
                    let screen_y = start_y + row;
                    // прекращаем рисовать если вышли за нижнюю границу экрана
                    if screen_y >= SCREEN_HEIGHT as u8 {
                        break;
                    }

                    // читаем байт строки спрайта из памяти по адресу I + row
                    let sprite_byte = self.memory[(self.i as usize) + row as usize];

                    // проходим по всем 8 битам в строке слева направо
                    for col in 0..8 {
                        let screen_x = start_x + col;

                        // прекращаем рисовать если вышли за правую границу экрана
                        if screen_x >= SCREEN_WIDTH as u8 {
                            break;
                        }

                        // проверяем конкретный бит (от 7го к 0му)
                        // маска 0x80 (1000_0000) сдвигает вправо на col
                        let sprite_pixel = (sprite_byte & (0x80 >> col)) != 0;

                        // рисуем если пиксель спрайта равен 1
                        if sprite_pixel {
                            let idx = (screen_y as usize) * SCREEN_WIDTH + (screen_x as usize);

                            // если пиксель уже был включен, фиксируем столкновение
                            if self.screen[idx] {
                                self.v[0xF] = 1;
                            }

                            // инвертируем пиксель на экране (XOR)
                            self.screen[idx] ^= true;
                        }
                    }
                }
            }

            // EX9E - Skip if key (SKP Vx)
            (0xE, _, 9, 0xE) => {
                let x = digit2 as usize;
                let key = (self.v[x] & 0x0F) as usize; // номер клавиши из регистра Vx
                if self.keypad[key] {
                    // если клавиша нажата
                    self.pc += 2;
                }
            }

            // EXA1 - Skip if !key (SKNP Vx)
            (0xE, _, 0xA, 1) => {
                let x = digit2 as usize;
                let key = (self.v[x] & 0x0F) as usize; // номер клавиши из регистра Vx
                if !self.keypad[key] {
                    // если клавиша не нажата
                    self.pc += 2;
                }
            }

            // FX07 - Timer: sets Vx to the value of the delay timer (dt)
            (0xF, _, 0, 7) => {
                let x = digit2 as usize;
                self.v[x] = self.dt;
            }

            // FX15 - Timer: sets the delay timer (dt) to the value in Vx
            (0xF, _, 1, 5) => {
                let x = digit2 as usize;
                self.dt = self.v[x];
            }

            // FX18 - Timer: sets the sound timer (st) to the value in Vx
            (0xF, _, 1, 8) => {
                let x = digit2 as usize;
                self.st = self.v[x];
            }

            // FX1E - Add to index
            (0xF, _, 1, 0xE) => {
                let x = digit2 as usize;
                self.i = self.v[x] as u16;
            }

            // FX0A - Get key
            (0xF, _, 0, 0xA) => {
                let x = digit2 as usize;
                let mut key_pressed = false;

                for (key_index, &is_down) in self.keypad.iter().enumerate() {
                    if is_down {
                        self.v[x] = key_index as u8;
                        key_pressed = true;
                        break;
                    }
                }

                if !key_pressed {
                    self.pc -= 2;
                }
            }

            // FX29 - Font Char
            (0xF, _, 2, 9) => {
                let x = digit2 as usize;
                let digit = self.v[x] & 0x0F;
                self.i = (0x000 + digit * 5) as u16;
            }

            // FX33: сохранить BCD-представление Vx в память по адресам I, I+1, I+2
            (0xF, _, 3, 3) => {
                let x = digit2 as usize;
                let value = self.v[x];
                let i = self.i as usize;

                self.memory[i] = value / 100;
                self.memory[i + 1] = (value / 10) % 10;
                self.memory[i + 2] = value % 10;
            }

            // FX55: store registers to memory
            (0xF, _, 5, 5) => {
                let x = digit2 as usize;
                let i = self.i as usize;
                self.memory[i..=i + x].copy_from_slice(&self.v[0..=x]);

                if !IS_SUPER_CHIP {
                    self.i += (x as u16) + 1;
                }
            }

            // FX65: load registers from memory
            (0xF, _, 6, 5) => {
                let x = digit2 as usize;
                let i = self.i as usize;
                self.v[0..=x].copy_from_slice(&self.memory[i..=i + x]);

                if !IS_SUPER_CHIP {
                    self.i += (x as u16) + 1;
                }
            }

            _ => unimplemented!("Неизвестный опкод: {:#06x}", op),
        }
    }

    // управление временем
    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }
    }

    // отдача картинки
    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    // прием нажатий
    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        if idx < 16 {
            self.keypad[idx] = pressed;
        }
    }

    pub fn is_sound_playing(&self) -> bool {
        self.st > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_emulator(opcodes: &[u16]) -> Emulator {
        let mut em = Emulator::new();
        let rom: Vec<u8> = opcodes.iter().flat_map(|op| op.to_be_bytes()).collect();
        em.load_rom(&rom);
        em
    }

    #[test]
    fn test_basic_opcodes() {
        // Программа: 610A (V1 = 10), 7105 (V1 +=5), 1200 (обратно на 0x200)
        let mut em = setup_emulator(&[0x610A, 0x7105, 0x1200]);

        em.tick();
        assert_eq!(em.v[1], 10);

        em.tick();
        assert_eq!(em.v[1], 15);

        em.tick();
        assert_eq!(em.pc, 0x200);
    }

    #[test]
    fn test_cxnn() {
        let mut em = Emulator::new();

        em.execute(0xC00F);
        assert!(em.v[0] <= 0x0F);
    }

    #[test]
    fn test_arithmetic() {
        // Программа: Vx + Vy, Vx + Vy, Vx - Vy, Vy - Vx
        let mut em = setup_emulator(&[0x8014, 0x8014, 0x8015, 0x8017]);

        // 10 + 20 = 30, V[0xF] = 0
        em.v[0] = 10;
        em.v[1] = 20;
        em.tick();
        assert_eq!(em.v[0], 30);
        assert_eq!(em.v[0xF], 0);

        // 250 + 10 = 4, V[0xF] = 1
        em.v[0] = 250;
        em.v[1] = 10;
        em.tick();
        assert_eq!(em.v[0], 4);
        assert_eq!(em.v[0xF], 1);

        // 5 - 10 = 251, V[0xF] = 0
        em.v[0] = 5;
        em.v[1] = 10;
        em.tick();
        assert_eq!(em.v[0], 251);
        assert_eq!(em.v[0xF], 0);

        // 10 - 5 = 5, V[0xF] = 1
        em.v[0] = 5;
        em.v[1] = 10;
        em.tick();
        assert_eq!(em.v[0], 5);
        assert_eq!(em.v[0xF], 1);
    }

    #[test]
    fn test_shifts() {
        // 8XY6, 8XYE
        let mut em = setup_emulator(&[0x8016, 0x801E]);

        em.v[0] = 0b0000_1011; // 11 lsb = 1
        em.v[1] = 0b0000_1010; // 10 lsb = 0
        em.tick();
        if IS_SUPER_CHIP {
            assert_eq!(em.v[0xF], 1);
            assert_eq!(em.v[0], 0b0000_0101) // после сдвига вправо
        } else {
            assert_eq!(em.v[0xF], 0);
            assert_eq!(em.v[0], 0b0000_0101) // после сдвига вправо
        }

        em.v[0] = 0b1100_1000; // 200 msb = 1
        em.v[1] = 0b0110_0100; // 100 msb = 0
        em.tick();
        if IS_SUPER_CHIP {
            assert_eq!(em.v[0xF], 1);
            assert_eq!(em.v[0], 0b1001_0000) // после сдвига влево
        } else {
            assert_eq!(em.v[0xF], 0);
            assert_eq!(em.v[0], 0b1100_1000) // после сдвига влево
        }
    }

    #[test]
    fn test_provisional_passes() {
        // 3XNN, 4XNN, 5XY0, 9XY0
        // 0000 for skip
        let mut em = setup_emulator(&[0x3016, 0x0000, 0x401E, 0x5010, 0x0000, 0x9010]);

        //for 0x3016
        em.v[0] = 0x16;
        let before_tick_pc = em.pc;
        em.tick();
        assert_eq!(em.pc - before_tick_pc, 4);

        // for 0x401E
        em.v[0] = 0x1E;
        let before_tick_pc = em.pc;
        em.tick();
        assert_eq!(em.pc - before_tick_pc, 2);

        // for 0x5010
        em.v[0] = 0x0001;
        em.v[1] = 0x0001;
        let before_tick_pc = em.pc;
        em.tick();
        assert_eq!(em.pc - before_tick_pc, 4);

        // for 0x9010
        em.v[0] = 0x0001;
        em.v[1] = 0x0001;
        let before_tick_pc = em.pc;
        em.tick();
        assert_eq!(em.pc - before_tick_pc, 2);
    }

    #[test]
    fn test_call_return() {
        // 2NNN, 00EE
        let mut em = setup_emulator(&[0x2300]);

        let before_call_pc = em.pc;
        println!("before call: {}", em.pc);

        //Call
        em.tick();
        assert_eq!(em.pc, 0x300);
        assert_eq!(em.stack.sp, 1);

        // 00EE
        em.memory[0x300] = 0x00;
        em.memory[0x301] = 0xEE;

        // Return
        em.tick();
        assert_eq!(em.pc, before_call_pc + 2);
        assert_eq!(em.stack.sp, 0);
    }

    #[test]
    fn test_rendering() {
        // DXYN
        let mut em = setup_emulator(&[0xD012, 0xD012]);

        em.v[0] = 10;
        em.v[1] = 10;
        em.tick();
        assert!(em.screen[10 * SCREEN_WIDTH + 10]);
        assert!(!em.screen[14 * SCREEN_WIDTH + 10]);
        assert_eq!(em.v[0xF], 0);

        em.tick();
        assert_eq!(em.v[0xF], 1);
    }
}
