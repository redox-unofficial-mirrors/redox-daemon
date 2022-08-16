#![feature(never_type)]

use syscall::{
    close,
    EIO,
    Error,
    exit,
    read,
    Result,
    write,
};

#[must_use = "Daemon::ready must be called"]
pub struct Daemon {
    write_pipe: usize,
}

fn e(result: libc::c_int) -> Result<usize> {
    if result == -1 {
        return Err(Error::new(unsafe { libc::__errno_location().read() }));
    }
    Ok(result as usize)
}

impl Daemon {
    pub fn new<F: FnOnce(Daemon) -> !>(f: F) -> Result<!> {
        let mut pipes: [libc::c_int; 2] = [0; 2];

        e(unsafe { libc::pipe(pipes.as_mut_ptr()) })?;

        let [read_pipe, write_pipe] = pipes.map(|c_fd| c_fd as usize);

        match e(unsafe { libc::fork() })? {
            // child
            0 => {
                let _ = close(read_pipe);

                f(Daemon {
                    write_pipe,
                });
            }
            // parent
            _ => {
                let _ = close(write_pipe);

                let mut data = [0];
                let res = read(read_pipe, &mut data);
                let _ = close(read_pipe);

                if res? == 1 {
                    exit(data[0] as usize)?;
                    unreachable!();
                } else {
                    Err(Error::new(EIO))
                }
            }
        }
    }

    pub fn ready(self) -> Result<()> {
        let res = write(self.write_pipe, &[0]);
        let _ = close(self.write_pipe);

        if res? == 1 {
            Ok(())
        } else {
            Err(Error::new(EIO))
        }
    }
}
