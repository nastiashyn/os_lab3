use core::ptr::null_mut;
use crate::{print, println};
use crate::vga_buf::SCREEN;
use pc_keyboard::DecodedKey;
use lazy_static::lazy_static;

const MAX_CHILDREN: usize = 10;
const MAX_DIR_NAME: usize = 10;


lazy_static! {
    static ref SH: spin::Mutex<Shell> = spin::Mutex::new({
        let mut sh = Shell::new();
        sh
    });
}

pub fn handle_keyboard_interrupt(key: DecodedKey) {
    match key {
        DecodedKey::Unicode(c) => SH.lock().on_key_pressed(c as u8),
        DecodedKey::RawKey(rk) => {}
    }
}

struct Dirs {
    dirs: [Dir; 100],
    next_dir: usize
}

#[derive(Debug, Clone, Copy)]
struct Dir {
    index: usize,
    name: [u8; MAX_DIR_NAME],
    parent_index: usize,
    child_count: usize,
    child_indexes: [usize; MAX_CHILDREN],
}


struct Shell {
    buf: [u8; 80],
    buf_len: usize,
    dirs: Dirs,
    current_dir: Dir
}

impl Shell {
    pub fn new() -> Shell {
        let start_dir = Dir {
            index: 0,
            name: [b'r', b'o', b'o', b't', b' ', b' ', b' ', b' ', b' ', b' '],
            parent_index: 0,
            child_count: 0,
            child_indexes: [0; MAX_CHILDREN]
        };

        let mut shell = Shell {
            buf: [0; 80],
            buf_len: 0,
            dirs: Dirs {
                dirs: [Dir {
                    index: 0,
                    name: [b' '; MAX_DIR_NAME],
                    parent_index: 0,
                    child_count: 0,
                    child_indexes: [0; MAX_CHILDREN]
                }; 100],
                next_dir: 1
            },
            current_dir: start_dir
        };

        shell.dirs.dirs[0] = shell.current_dir;

        return shell;
    }

    fn compare_commands(variant: &str, command: [u8; 80]) -> bool {
        let mut is_correct = true;
        let mut i = 0;

        for symbol in variant.bytes() {
            if symbol != command[i] {
                is_correct = false;
                return is_correct;
            }

            i += 1;
        }

        if command[i] != b' ' {
            is_correct = false;
        }

        return is_correct;
    }

    fn pick_command(&mut self, command: [u8; 80], argument: [u8; 10]) {
        println!();

        if Self::compare_commands("cur_dir", command) {
            self.cur_dir();
            println!()
        } else if Self::compare_commands("make_dir", command) {
            self.make_dir(argument);
        } else if Self::compare_commands("change_dir", command) {
            self.change_dir(argument);
        } else if Self::compare_commands("remove_dir", command) {
            self.remove_dir(argument);
        } else if Self::compare_commands("dir_tree", command) {
            self.dir_tree();
        } else if Self::compare_commands("clear", command) {
            SCREEN.lock().clear();
        } else {
            println!("No such command!");
        }
    }

    fn cur_dir(&self) {
        print!("/");
        for symbol in self.current_dir.name {
            print!("{}", symbol as char);
        }
    }

    fn make_dir(&mut self, argument: [u8; 10]) {
        if argument[0] == b' ' {
            println!("Put not empty argument!");
            return;
        }

        if self.find_dir(argument) != 0 {
            println!("Such dir is already exists!");
            return;
        }

        let next_dir_index = self.dirs.next_dir;

        if next_dir_index < 100 {
            let parent_id = self.current_dir.index;

            let child = Dir {
                index: next_dir_index,
                name: argument,
                parent_index: parent_id,
                child_count: 0,
                child_indexes: [0; MAX_CHILDREN]
            };

            self.dirs.dirs[next_dir_index] = child;
            self.dirs.next_dir += 1;
            self.current_dir.child_indexes[self.current_dir.child_count] = child.index;
            self.current_dir.child_count += 1;
            self.dirs.dirs[self.current_dir.index] = self.current_dir;

            print!("Success! Created dir ");

            for symbols in argument {
                print!("{}", symbols as char);
            }

            println!();
        }
    }

    fn change_dir(&mut self, name: [u8; 10]) {
        if name[0] == b' ' {
            println!("Put not empty argument!");
            return;
        }

        if name[0] == b'.' {
            self.current_dir = self.dirs.dirs[self.current_dir.parent_index];
            return;
        }

        let dir_index = self.find_dir(name);

        if dir_index == 0 {
            println!("No such directory!")
        } else {
            self.dirs.dirs[self.current_dir.index] = self.current_dir;
            self.current_dir = self.dirs.dirs[dir_index];
            print!("Success moved to dir ");

            for symbols in name {
                print!("{}", symbols as char);
            }

            println!();
        }
    }

    fn remove_dir(&mut self, name: [u8; 10]) {
        if name[0] == b' ' {
            println!("Put not empty argument!");
            return;
        }

        let dir_index = self.find_dir(name);

        if dir_index == 0 {
            println!("No such directory!")
        } else {
            let mut n: usize = 0;
            for index in self.current_dir.child_indexes {
                if index == dir_index {
                    self.current_dir.child_indexes[n] = 0;
                    self.move_child_indexes(n);
                    break;
                }

                n += 1;
            }

            self.current_dir.child_count -= 1;

            self.dirs.dirs[self.current_dir.index] = self.current_dir;

            for child_indexes in self.dirs.dirs[dir_index].child_indexes {
                if child_indexes == 0 {
                    break;
                }

                self.clear_dir_by_index(child_indexes);
            }

            self.clear_dir_by_index(dir_index);

            print!("Success removed dir ");

            for symbols in name {
                print!("{}", symbols as char);
            }

            println!();
        }
    }

    fn dir_tree(&mut self) {
        self.unwrap_dir(self.current_dir.index, 0);
    }

    fn unwrap_dir(&mut self, index: usize, level: u8) {
        let mut i = 0;

        while i < level {
            print!("{}", "    ");
            i += 1;
        }

        if self.dirs.dirs[index].child_count == 0 {
            print!("/");
            for symbol in self.dirs.dirs[index].name {
                print!("{}", symbol as char);
            }

            println!();
        } else {
            print!("/");
            for symbol in self.dirs.dirs[index].name {
                print!("{}", symbol as char);
            }

            println!();

            let mut n = 0;
            while n < self.dirs.dirs[index].child_count {
                self.unwrap_dir(self.dirs.dirs[index].child_indexes[n], level + 1);
                n += 1;
            }

        }
    }

    fn move_child_indexes(&mut self, start: usize) {
        let mut iter = start + 1;

        while iter < MAX_CHILDREN {
            self.current_dir.child_indexes[iter - 1] = self.current_dir.child_indexes[iter];
            iter += 1;
        }
    }

    fn clear_dir_by_index(&mut self, index: usize) {
        self.dirs.dirs[index] = Dir {
            index: 0,
            name: [b' '; MAX_DIR_NAME],
            parent_index: 0,
            child_count: 0,
            child_indexes: [0; MAX_CHILDREN]
        };
    }

    fn find_dir(&mut self, name: [u8; 10]) -> usize {
        for dir in self.current_dir.child_indexes {
            let mut is_correct = true;

            let mut i = 0;

            for symbol in self.dirs.dirs[dir].name {
                if symbol != name[i] {
                    is_correct = false;
                    break;
                }

                i += 1;
            }

            if is_correct {
                return dir;
            }
        }

        return 0;
    }

    pub fn on_key_pressed(&mut self, key: u8) {
        match key {
            b'\n' => {
                let command = parse_command(self.buf, self.buf_len);
                let argument = parse_argument(self.buf, self.buf_len);

                self.pick_command(command, argument);
                self.buf_len = 0;
            }
            8 => {
                self.buf[self.buf_len] = 0;
                self.buf_len -= 1;
                SCREEN.lock().clear_last();
            }
            _ => {
                self.buf[self.buf_len] = key;
                self.buf_len += 1;
                print!("{}", key as char);
            }
        }
    }
}

pub fn parse_command(buf: [u8; 80], buf_len: usize) -> [u8; 80] {
    let mut command: [u8; 80] = [b' '; 80];

    let mut n = 0;

    while buf[n] != b' ' && n < buf_len {
        command[n] = buf[n];
        n += 1;
    }

    return command;
}

pub fn parse_argument(buf: [u8; 80], buf_len: usize) -> [u8; 10] {
    let mut argument: [u8; 10] = [b' '; 10];

    let mut n = 0;

    while buf[n] != b' ' && n < buf_len {
        n += 1;
    }

    n += 1;

    let mut i = 0;

    while n < buf_len && buf[n] != b' ' {
        argument[i] = buf[n];
        n += 1;
        i += 1;
    }

    return argument;
}