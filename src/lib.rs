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

#[cfg(feature = "scheme")]
pub fn get_scheme_auth(auth_socket: usize, id: u64) -> Result<syscall::Auth> {
    let _ = syscall::write(auth_socket, &u64::to_ne_bytes(id))?;
    let mut bytes = [0_u8; core::mem::size_of::<syscall::Auth>()];
    let _ = syscall::read(auth_socket, &mut bytes)?;
    Ok(*bytemuck::from_bytes(&bytes))
}
#[cfg(feature = "scheme")]
pub fn response_fevent_packet(flags: syscall::EventFlags, id: usize, value: usize) -> [syscall::Cqe; 2] {
    const EVENT_SHIFT: u32 = usize::BITS - 8;
    const EVENT_MASK: u64 = (1_u64 << EVENT_SHIFT) - 1;

    [
        syscall::data::Cqe {
            user_data: 0,
            result: (syscall::number::SchemeResponseOp::Fevent as u64) << EVENT_SHIFT | (flags.bits() as u64 & EVENT_MASK),
        },
        syscall::data::Cqe {
            user_data: id as u64,
            result: value as u64,
        },
        // TODO: The kernel used to ignore the `d` field when writing SYS_FEVENT.
        // If we made response codes 8 bits, the flags could be 56 (or 24) bits
        // depending on pointer size.
    ]
}
