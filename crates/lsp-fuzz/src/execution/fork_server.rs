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

const INVALID_MAP_SIZE_MSG: &str =
    "AFL_MAP_SIZE is not set and fuzzing target reports that the required size is very large. \
     Solution: Run the fuzzing target stand-alone with the environment variable AFL_DEBUG=1 set \
     and set the value for __afl_final_loc in the AFL_MAP_SIZE environment variable for afl-fuzz.";

const INVALID_MAP_ADDR_MSG: &str =
"The fuzzing target reports that hardcoded map address might be the reason the mmap of the shared memory failed. \
Solution: recompile the target with either afl-clang-lto and do not set AFL_LLVM_MAP_ADDR or recompile with afl-clang-fast.";

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
            FS_ERROR_MAP_SIZE =>  libafl::Error::unknown(INVALID_MAP_SIZE_MSG) ,
            FS_ERROR_MAP_ADDR => libafl::Error::unknown( INVALID_MAP_ADDR_MSG ),
            FS_ERROR_SHM_OPEN => libafl::Error::unknown("The fuzzing target reports that the shm_open() call failed."),
            FS_ERROR_SHMAT => libafl::Error::unknown("The fuzzing target reports that the shmat() call failed."),
            FS_ERROR_MMAP => libafl::Error::unknown("The fuzzing target reports that the mmap() call to the shared memory failed."),
            FS_ERROR_OLD_CMPLOG => libafl::Error::unknown("The -c cmplog target was instrumented with an too old AFL++ version, you need to recompile it."),
            FS_ERROR_OLD_CMPLOG_QEMU => libafl::Error::unknown("The AFL++ QEMU/FRIDA loaders are from an older version, for -c you need to recompile it."),
            code => libafl::Error::unknown(format!("Unknown error code {code} from fuzzing target!")),
        };
        Err(err)
    } else {
        Ok(())
    }
}
