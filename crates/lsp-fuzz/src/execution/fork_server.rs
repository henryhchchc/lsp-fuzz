use libafl::{executors::Forkserver as ForkServer, mutators::Tokens, observers::MapObserver};
use libafl_bolts::Truncate;
use tracing::info;

use crate::utils::ResultExt;

const FSRV_NEW_VERSION_MIN: u32 = 1;
const FSRV_NEW_VERSION_MAX: u32 = 1;

pub(super) fn check_version(handshake_msg: i32) -> Result<(), libafl::Error> {
    if is_old_forkserver(handshake_msg) {
        Err(libafl::Error::unknown(
            "Old fork server model is used by the target, it is nolonger supportted",
        ))?;
    }

    let version: u32 = handshake_msg as u32 - 0x41464c00_u32;
    match version {
        FSRV_NEW_VERSION_MIN..=FSRV_NEW_VERSION_MAX => Ok(()),
        0 => Err(libafl::Error::unknown(
            "Fork server version is not assigned, this should not happen. Recompile target.",
        ))?,
        _ => Err(libafl::Error::unknown(
            "Fork server version is not supported. Recompile the target.",
        ))?,
    }
}

fn is_old_forkserver(handshake_msg: i32) -> bool {
    !(0x41464c00..0x41464cff).contains(&handshake_msg)
}

pub(super) fn check_handshake_error_bits(handshake_msg: i32) -> Result<(), libafl::Error> {
    #[allow(clippy::cast_possible_wrap)]
    const FS_NEW_ERROR: i32 = 0xeffe0000_u32 as _;

    const FS_ERROR_MAP_SIZE: i32 = 1 << 0;
    const FS_ERROR_MAP_ADDR: i32 = 1 << 1;
    const FS_ERROR_SHM_OPEN: i32 = 1 << 2;
    const FS_ERROR_SHMAT: i32 = 1 << 3;
    const FS_ERROR_MMAP: i32 = 1 << 4;
    const FS_ERROR_OLD_CMPLOG: i32 = 1 << 5;
    const FS_ERROR_OLD_CMPLOG_QEMU: i32 = 1 << 6;

    if (handshake_msg & FS_NEW_ERROR) == FS_NEW_ERROR {
        let error_code = handshake_msg & 0x0000ffff;
        let err = match error_code {
            FS_ERROR_MAP_SIZE =>  libafl::Error::unknown(
                "AFL_MAP_SIZE is not set and fuzzing target reports that the required size is very large. \
                      Solution: Run the fuzzing target stand-alone with the environment variable AFL_DEBUG=1 set \
                      and set the value for __afl_final_loc in the AFL_MAP_SIZE environment variable for afl-fuzz."
            ) ,
            FS_ERROR_MAP_ADDR => libafl::Error::unknown(
                "The fuzzing target reports that hardcoded map address might be the reason the mmap of the \
                  shared memory failed. Solution: recompile the target with either afl-clang-lto and do not \
                  set AFL_LLVM_MAP_ADDR or recompile with afl-clang-fast."
            ),
            FS_ERROR_SHM_OPEN => libafl::Error::unknown(
                "The fuzzing target reports that the shm_open() call failed."
            ),
            FS_ERROR_SHMAT => libafl::Error::unknown("The fuzzing target reports that the shmat() call failed."),
            FS_ERROR_MMAP => libafl::Error::unknown(
                "The fuzzing target reports that the mmap() call to the shared memory failed."
            ),
            FS_ERROR_OLD_CMPLOG => libafl::Error::unknown(
                "The -c cmplog target was instrumented with an too old AFL++ version, you need to recompile it."
            ),
            FS_ERROR_OLD_CMPLOG_QEMU => libafl::Error::unknown(
                "The AFL++ QEMU/FRIDA loaders are from an older version, for -c you need to recompile it."
            ),
            code => libafl::Error::unknown(format!("Unknown error code {code} from fuzzing target!")),
        };
        Err(err)
    } else {
        Ok(())
    }
}

const FS_NEW_OPT_MAPSIZE: i32 = 1 << 0;
const FS_NEW_OPT_SHDMEM_FUZZ: i32 = 1 << 1;
const FS_NEW_OPT_AUTODICT: i32 = 1 << 11;

pub(super) fn initialize<MO, A, SHM>(
    fork_server: &mut ForkServer,
    map_observer: &mut A,
    test_case_shm: &Option<SHM>,
    auto_tokens: Option<&mut Tokens>,
) -> Result<(), libafl::Error>
where
    A: AsRef<MO> + AsMut<MO>,
    MO: MapObserver + Truncate,
{
    let handshake_msg = fork_server
        .read_st()
        .afl_context("Oops the fork server fucked up.")?;
    check_handshake_error_bits(handshake_msg)?;
    check_version(handshake_msg)?;
    let handshake_response = (handshake_msg as u32 ^ 0xffffffff) as i32;
    fork_server
        .write_ctl(handshake_response)
        .afl_context("Fail to write handshake response to forkserver")?;
    let fsrv_options = fork_server
        .read_st()
        .afl_context("Fail to read options from forkserver")?;
    if fsrv_options & FS_NEW_OPT_MAPSIZE == FS_NEW_OPT_MAPSIZE {
        let fsrv_map_size = fork_server
            .read_st()
            .afl_context("Failed to read map size from forkserver")?;

        let fsrv_map_size = fsrv_map_size as usize;

        match map_observer.as_ref().len() {
            map_size if map_size > fsrv_map_size => {
                map_observer.as_mut().truncate(fsrv_map_size);
                info!(new_size = fsrv_map_size, "Coverage map truncated");
            }
            map_size if map_size < fsrv_map_size => {
                Err(libafl::Error::illegal_argument(format!(
                    "The map size is too small. {fsrv_map_size} is required for the target."
                )))?;
            }
            map_size if map_size == fsrv_map_size => {}
            _ => unreachable!("Garenteed by the match statement above."),
        }
    };
    if fsrv_options & FS_NEW_OPT_SHDMEM_FUZZ != 0 && test_case_shm.is_none() {
        Err(libafl::Error::unknown(
            "Target requested sharedmem fuzzing, but you didn't prepare shmem",
        ))?;
    }
    if fsrv_options & FS_NEW_OPT_AUTODICT != 0 {
        // Here unlike shmem input fuzzing, we are forced to read things
        // hence no self.autotokens.is_some() to check if we proceed
        let autotokens_size = fork_server
            .read_st()
            .afl_context("Failed to read autotokens size from forkserver")?;

        let tokens_size_max = 0xffffff;

        if !(2..=tokens_size_max).contains(&autotokens_size) {
            let message = format!(
                "Autotokens size is incorrect, expected 2 to {tokens_size_max} (inclusive), \
                    but got {autotokens_size}. Make sure your afl-cc verison is up to date."
            );
            Err(libafl::Error::illegal_state(message))?;
        }
        info!(size = autotokens_size, "AUTODICT detected.");
        let auto_tokens_buf = fork_server
            .read_st_of_len(autotokens_size as usize)
            .afl_context("Failed to load autotokens")?;
        if let Some(t) = auto_tokens {
            info!("Updating autotokens.");
            t.parse_autodict(&auto_tokens_buf, autotokens_size as usize);
        }
    }
    let aflx = fork_server
        .read_st()
        .afl_context("Reading from forkserver failed")?;
    if aflx != handshake_msg {
        let message = format!("Error in forkserver communication ({aflx:?}=>{handshake_msg:?})");
        Err(libafl::Error::unknown(message))?;
    }
    Ok(())
}
