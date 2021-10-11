use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::os::unix::io::{AsRawFd, RawFd};
use nix::sys::select::{FdSet, select};
use std::collections::HashMap;

fn main() {
    // spawn a process which prints to both stdout and stderr, just for testing
    let (cmd, args) = ("curl", ["https://www.auscert.org.au"]);
    #[cfg(debug_assertions)] { println!("Command: \"{}\" {:?}", cmd, args); }
    let process = match Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() {
        Err(why) => panic!("couldn't spawn printer: {}", why),
        Ok(process) => process,
    };

    // FDs that select() should read.
    // NB: select() mutates its input FDSets.
    // so, if we want to repeatedly scan each FD, we need to pass it a fresh FDSet each time.
    // we end up maintaining a 'master' set and cloning it to pass a clone to select().
    let mut master_fd_set = FdSet::new(); 

    // get handles on the child's stdout and stderr and put them into the master_fd_set

    // do NOT join these two lines as as_raw_fd will take a reference to the ChildStdout, not a
    // value, so ChildStdout will be referenceless, therefore dropped, and will close the FD on its way out.
    // this is kind of academic anyway since we need to read into it later
    let child_stdout = process.stdout.expect("Could not open child stdout");
    let child_stdout_raw_fd = child_stdout.as_raw_fd();
    #[cfg(debug_assertions)] { dbg!(child_stdout_raw_fd); }
    master_fd_set.insert(child_stdout_raw_fd);

    let child_stderr = process.stderr.expect("Could not open child stderr");
    let child_stderr_raw_fd = child_stderr.as_raw_fd();
    #[cfg(debug_assertions)] { dbg!(child_stderr_raw_fd); }
    master_fd_set.insert(child_stderr_raw_fd);

    // map RawFd to a ChildStd(out|err) for reading
    // because it's easier to read the Child* structs than open a File from the RawFd
    let mut stream_map: HashMap<RawFd, Box<dyn Read>> = HashMap::new();
    stream_map.insert(child_stdout_raw_fd, Box::new(child_stdout));
    stream_map.insert(child_stderr_raw_fd, Box::new(child_stderr));

    // loop variables
    let mut buf = String::new(); // temporary growable buffer. needs to be cleared between reads or it'll just get appended to
    let mut fds_to_remove: Vec<RawFd> = vec![]; // FDs to remove from the select() set after each loop

    // main loop. iterate over the 'open' FDs in the master set, removing them as the other end
    // closes them, until they're all done.
    while master_fd_set.fds(None).count() > 0 {
        let _ = select(None, &mut master_fd_set.clone(), None, None, None); // blocks until one or more is ready
        for fd in master_fd_set.fds(None) {
            #[cfg(debug_assertions)] { println!("fd {:?} is ready for I/O", fd); }
            let stream = stream_map.get_mut(&fd).unwrap();
            let bytes_read = match stream.read_to_string(&mut buf) {
                Err(why) => panic!("Error reading from stdout: {}", why),
                Ok(bytes_read) => bytes_read,
            };
            #[cfg(debug_assertions)] { println!("Read {:?} bytes OK", bytes_read); }
            if bytes_read == 0 {
                // signals that the pipe was closed at the other end. if it's possible to close our
                // end, here is where we'd do it, but it doesn't seem to be available
                fds_to_remove.push(fd);
                #[cfg(debug_assertions)] { println!("fd {} sealed its fate", fd); }
            } else {
                let colour = if fd == child_stdout_raw_fd { "\x1B[32m" }
                else if fd == child_stderr_raw_fd { "\x1B[31m" }
                else { "\x1B0m" };
                print!("{}{}\x1B[0m", colour, buf);
            }
            buf.clear();
            std::io::stdout().flush().expect("Error flushing stdout");
        }
        for fd in &fds_to_remove {
            #[cfg(debug_assertions)] { println!("Dropping closed FD {}", fd); }
            master_fd_set.remove(*fd);
        }
        fds_to_remove.clear();
    }
    #[cfg(debug_assertions)] { println!("All done"); }
}
