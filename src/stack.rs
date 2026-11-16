// стек вызовов
pub struct Stack {
    data: [u16; 16],
    pub sp: u8, // указатель стека
}

impl Stack {
    pub fn new() -> Self {
        Self {
            data: [0; 16],
            sp: 0,
        }
    }

    pub fn push(&mut self, val: u16) {
        self.data[self.sp as usize] = val;
        self.sp += 1;
    }

    pub fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.data[self.sp as usize]
    }
}
