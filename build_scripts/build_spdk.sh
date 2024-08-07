#!/usr/bin/env bash

set -o pipefail

# Directory of this script.
SCRIPT_DIR=$(dirname "$(realpath "$0")")
export SCRIPT_DIR

# This script's logging dir.
LOG_DIR=$(realpath "$SCRIPT_DIR/../build_logs")
export LOG_DIR

export SPDK_ROOT_DIR=${SPDK_ROOT_DIR:-""}           # Root of SPDK sources.
export SPDK_VERSION="24.05"                         # SPDK version (currently, informative only).

export BUILD_TYPE="debug"
export TARGET_PLATFORM="x86_64-unknown-linux-gnu"
export CROSS_PREFIX=
export TARGET_PLATFORM_x86_64="yes"
export WITH_FIO="system"
export FIO_DST=""
export LOG_MODE="tee"                                      # no|tee|silent
export LOG_MODE_EXPLICIT="no"
export DRY_RUN="no"
export VERBOSE=0

# ANSI color codes.
export RESET_COLOR='\033[0m'
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[0;33m'
export CYAN='\033[0;36m'
export WHITE='\033[0;37m'
export WHITE_HIGH='\033[0;97m'

export MSG_PREFIX="${WHITE_HIGH}SPDK build =>${RESET_COLOR}"
export MSG_COL_INFO=$GREEN
export MSG_COL_DBG=
export MSG_COL_WARN=$YELLOW
export MSG_COL_ERR=$RED
export MSG_NC=$RESET_COLOR

# SPDK configure arguments.
CONFIGURE_ARGS=(
    "--without-shared"
    "--with-uring"
    "--without-uring-zns"
    "--without-nvme-cuse"
    "--without-fuse"
    "--disable-unit-tests"
    "--disable-tests"
    "--with-rdma"
)

MAKE_DEFAULT_ARGS="-j"                              # Default args for invoking GNU make
INSTALL_DIR=""

# Prints usage.
function usage() {
    echo '''
Usage: build_spdk.sh [OPTIONS] <COMMAND> [-- <ARGS>]

Options:
    -b, --build-type <BT>       Build type
    -t, --target <TGT>          Target platform (see below)
    --cross-prefix <TGT>        Prefix for cross compilation
    -s, --with-spdk <PATH>      SPDK root directory
    --with-fio <FIO>            Use given FIO path
    --without-fio               Build without FIO
    --with-fio-dst <PATH>       FIO install destination
    --log <no|tee*|silent>      Output logging options
    --no-log                    Same as "--log no"
    -n, --dry-run               Dry run
    -v, --verbose               Enable verbose
    -h, --help                  Print help

Commands:
    configure                   Configures SPDK
    make                        Compiles SPDK
    clean                       Clean SPDK with git clean
    install <PATH>              Install SPDK at the given path
    fmt                         Runs check format script
    sha256                      Determine SHA256 for the latest SPDK commit
    help                        Print help

Build types:
    debug*
    release

Target platforms:
    x86_64-unknown-linux-gnu*
    aarch64-unknown-linux-gnu

(* - default option)
'''
    exit "$1"
}

# Prints an info message.
function msg_info() {
    echo -e "$MSG_PREFIX$MSG_COL_INFO $* $MSG_NC"
}

export -f msg_info

# Print a debug message.
function msg_debug() {
    if [[ "$VERBOSE" -ge 1 ]]
    then
        echo -e "$MSG_PREFIX$MSG_COL_DBG $* $MSG_NC"
    fi
}

export -f msg_debug

# Print a warning message.
function msg_warn() {
    echo -e "$MSG_PREFIX$MSG_COL_WARN Warning: $* $MSG_NC"
}

export -f msg_warn

# Print an error message.
function msg_error() {
    echo -e "$MSG_PREFIX$MSG_COL_ERR Error: $* $MSG_NC"
}

export -f msg_error

# Prints an error message for script options or commands.
function script_error() {
    echo -e "$* $MSG_NC" > /dev/stderr
}

export -f script_error


# Prepares log files and writes the given commands args to the command arg log file.
function prepare_log() {
    if [[ $# -ne 3 ]]
    then
        script_error "Bad number of args for prepare_log: $#"
        exit 1
    fi

    # Log file for command output.
    OUTPUT_LOG_FILE="$LOG_DIR/$1"
    shift

    # Log file for command arguments.
    CMD_ARGS_LOG_FILE="$LOG_DIR/$1"
    shift

    mkdir -p "$LOG_DIR"
    rm -f "$OUTPUT_LOG_FILE" "$CMD_ARGS_LOG_FILE"
    echo "$1" > "$CMD_ARGS_LOG_FILE"
}

export -f prepare_log

# TODO
function exec_tool() {
    if [[ $# -lt 2 ]]
    then
        script_error "Bad number of args for exec_tool: $#"
        exit 1
    fi

    # Client-faced tool name.
    NAME="$1"
    shift

    OUTPUT_LOG_FILE="${NAME}.log"
    CMD_ARGS_LOG_FILE="${NAME}.cmd"

    # Log command name + command args.
    LOG_CMD_ARGS="$*"

    # Tool's executable
    TOOL=$1
    shift

    case $LOG_MODE in
        "no")
            $TOOL $@
            ;;
        "tee")
            prepare_log "$OUTPUT_LOG_FILE" "$CMD_ARGS_LOG_FILE" "$LOG_CMD_ARGS"
            $TOOL $@ | tee "$OUTPUT_LOG_FILE"
            return $?
            ;;
        "silent")
            prepare_log "$OUTPUT_LOG_FILE" "$CMD_ARGS_LOG_FILE" "$LOG_CMD_ARGS"
            $TOOL $@ > "$OUTPUT_LOG_FILE"
            R=$?
            if [[ $R -ne 0 ]]
            then
                cat "$OUTPUT_LOG_FILE"
            fi
            ;;
        *)
            script_error "Bad log mode: ${LOG_MODE}"
            exit 1
    esac
}

export -f exec_tool

# TODO
function silent_pushd() {
    pushd "$1" > /dev/null || exit 1
}

export -f silent_pushd

# TODO
function silent_popd() {
    popd > /dev/null || exit 1
}

export -f silent_popd


# Detects system FIO installation.
function detect_fio() {
    if [[ "$WITH_FIO" == "system" ]]
    then
        if FIO_BIN=$(which fio 2>/dev/null)
        then
            FIO_BIN=$(realpath "$FIO_BIN")
            msg_info "Found system FIO binary:" "$FIO_BIN"
            WITH_FIO=$(dirname "$FIO_BIN")
            WITH_FIO=$(dirname "$WITH_FIO")
            msg_info "System FIO installation directory:" "$WITH_FIO"
        else
            msg_warn "No fio installation found"
            WITH_FIO=""
        fi
    fi

    # TODO: test if fio installation dir is valid
    #if ! stat "$WITH_FIO/include";
    #then
    #    msg_warn "No fio include directory found"
    #    WITH_FIO=""
    #fi
}

function cmd_configure() {
    #
    # Making configure options.
    #

    # Build type options.
    msg_info "Build type: $BUILD_TYPE"
    if [[ "$BUILD_TYPE" == "debug" ]]
    then
        CONFIGURE_ARGS+=("--enable-debug")
    fi

    # Target platform options.
    msg_info "Target platform: $TARGET_PLATFORM"
    case $TARGET_PLATFORM in
        "x86_64-unknown-linux-gnu")
            CONFIGURE_ARGS+=("--target-arch=nehalem" "--without-crypto")
            ;;
        "aarch64-unknown-linux-gnu")
            CONFIGURE_ARGS+=(" --target-arch=armv8-a+crypto")
            ;;
    esac

    # Cross compilation.
    if [[ -n $CROSS_PREFIX ]]
    then
        CONFIGURE_ARGS+=("--cross-prefix=$CROSS_PREFIX")
    fi

    # FIO plugin option.
    detect_fio
    if [[ -n "$WITH_FIO" ]]
    then
        msg_info "With FIO: $WITH_FIO"
        CONFIGURE_ARGS+=("--with-fio=$WITH_FIO")
    else
        msg_info "Building without FIO plugin"
        CONFIGURE_ARGS+=("--without-fio")
    fi

    #
    # Configure logic starts here.
    #

    # Convert args array to string.
    CFG_CMD_ARGS="${CONFIGURE_ARGS[*]}"

    msg_info "Will run configure with arguments:" "$MSG_NC$CFG_CMD_ARGS"

    if [[ "$DRY_RUN" == "yes" ]]
    then
        msg_info "Dry run is enabled, skipping configure"
        return 0
    fi

    msg_info "Running configure ..."

    # Configure script needs asm (AS shell var) to be defined.
    export AS=nasm

    silent_pushd "$SPDK_ROOT_DIR"
    exec_tool "configure" ./configure "$CFG_CMD_ARGS"
    R=$?
    silent_popd

    if [[ $R -ne 0 ]]
    then
        msg_error "Configure: failed"
        return $R
    fi

    msg_info "Configure: ok"
    return 0
}

function cmd_make() {
    if [[ $# -gt 0 ]]
    then
        MAKE_ARGS=$*
    else
        msg_debug "Using default make args:" $MAKE_DEFAULT_ARGS
        MAKE_ARGS=$MAKE_DEFAULT_ARGS
    fi

    # Target platform options.
    if [[ "$TARGET_PLATFORM" == "aarch64-unknown-linux-gnu" ]]
    then
        export DPDKBUILD_FLAGS="-Dplatform=generic"
    fi

    MAKE_MSG="'make $MAKE_ARGS'"
    msg_debug "Will run make with arguments:" $MAKE_ARGS

    if [[ "$DRY_RUN" == "yes" ]]
    then
        msg_info "Dry run is enabled, skipping make"
        return 0
    fi

    msg_info "Running $MAKE_MSG ..."
    silent_pushd "$SPDK_ROOT_DIR"
    exec_tool "make" make "$MAKE_ARGS"
    R=$?
    silent_popd

    if [[ $R -ne 0 ]]
    then
        msg_error "Running $MAKE_MSG: failed"
        return $R
    fi

    msg_info "Running $MAKE_MSG: ok"
    return 0
}

function cmd_install() {
    if [[ -z "$INSTALL_DIR" ]]
    then
        script_error "Install directory must be specified"
        usage 1
    fi

    out=$(realpath "$INSTALL_DIR")

    msg_info "Will install SPDK to $out ..."

    if [[ "$DRY_RUN" == "yes" ]]
    then
        msg_info "Dry run is enabled, skipping install"
        return 0
    fi

    mkdir -p "$out/lib/pkgconfig"
    mkdir -p "$out/bin"

    silent_pushd "$SPDK_ROOT_DIR"

    pushd include || exit 1
    msg_info "Installing includes..."
    find . -type f -name "*.h" -exec install -vD "{}" "$out/include/{}" \;
    popd || exit 1

    pushd lib || exit 1
    msg_info "Installing library includes..."
    find . -type f -name "*.h" -exec install -vD "{}" "$out/include/spdk/lib/{}" \;
    popd || exit 1

    # copy private headers from bdev modules needed for creating of bdevs
    pushd module || exit 1
    msg_info "Installing bdev module includes..."
    find . -type f -name "*.h" -exec install -vD "{}" "$out/include/spdk/module/{}" \;
    popd || exit 1

    find . -executable -type f -name 'bdevperf' -exec install -vD "{}" "$out/bin" \;

    # copy libraries
    msg_info "Installing libs and pkgconfig..."
    install -v build/lib/*.a                   "$out/lib/"
    install -v build/lib/pkgconfig/*.pc        "$out/lib/pkgconfig/"
    install -v dpdk/build/lib/*.a              "$out/lib/"
    install -v dpdk/build/lib/pkgconfig/*.pc   "$out/lib/pkgconfig/"

    if [[ "$TARGET_PLATFORM_x86_64" == "yes" ]]
    then
        msg_info "Installing ISA-L libs and pkgconfig..."
        install -v isa-l/.libs/*.a                 "$out/lib/"
        install -v isa-l/*.pc                      "$out/lib/pkgconfig/"
        install -v isa-l-crypto/.libs/*.a          "$out/lib/"
        install -v isa-l-crypto/*.pc               "$out/lib/pkgconfig/"
    fi

    # Fix paths in pkg config files.
    build_dir=$(pwd)
    for i in $(ls $out/lib/pkgconfig/*.pc); do
        msg_info "Fixing pkg config paths in '$i' ..."
        sed -i "s,$build_dir/build/lib,$out/lib,g" "$i"
        sed -i "s,$build_dir/dpdk/build,$out,g" "$i"
        sed -i "s,$build_dir/intel-ipsec-mb/lib,$out/lib,g" "$i"
        sed -i "s,$build_dir/isa-l/.libs,$out/lib,g" "$i"
        sed -i "s,$build_dir/isa-l-crypto/.libs,$out/lib,g" "$i"
        sed -i "s,prefix\=/usr/local,prefix\=$out,g" "$i"
    done

    if [[ -n "$WITH_FIO" ]]
    then
        if [[ -n "$WITH_FIO_DST" ]]
        then
            FIO_DST="$WITH_FIO_DST"
        else
            FIO_DST="$out/fio"
        fi
        msg_info "Installing SPDK FIO plugin into $FIO_DST"
        mkdir -p "$FIO_DST"
        install -v build/fio/spdk_* "$FIO_DST/"
    fi

    if [[ "$BUILD_TYPE" == "debug" ]]
    then
        msg_info "Copying test files test dir to $out/test"
        cp -ar test "$out/test"
    fi

    silent_popd

    return 0
}

CMD=""
while [[ $# -gt 0 ]]
do
    C=$1
    shift

    case $C in
        "-b" | "--build-type")
            BUILD_TYPE=$1
            shift
            ;;
        "-t" | "--target")
            TARGET_PLATFORM=$1
            shift
            ;;
        "--cross-prefix")
            CROSS_PREFIX=$1
            shift
            ;;
        "-s" | "--with-spdk")
            SPDK_ROOT_DIR=$1
            shift
            ;;
        "--with-fio")
            WITH_FIO=$1
            shift
            ;;
        "--without-fio")
            WITH_FIO=""
            ;;
        "--with-fio-dst")
            WITH_FIO_DST=$1
            msg_info "With FIO install destination: $WITH_FIO_DST"
            shift
            ;;
        "--log")
            LOG_MODE=$1
            LOG_MODE_EXPLICIT="yes"
            shift
            ;;
        "--no-log")
            LOG_MODE="no"
            LOG_MODE_EXPLICIT="yes"
            ;;
        "-n" | "--dry-run")
            DRY_RUN="yes"
            msg_info "Dry run is enabled"
            ;;
        "-v" | "--verbose")
            VERBOSE=$(("$VERBOSE" + 1))
            msg_debug "Verbosity level is $VERBOSE"
            ;;
        "-h" | "--help" | "help")
            usage 0
            ;;
        "--")
            break
            ;;
        "configure" | "make" | "install" | "clean" | "fmt" | "sha256")
            if [[ -n $CMD ]]
            then
                script_error "Command is already given"
                usage 1
            fi

            CMD="$C"

            if [[ "$C" == "install" ]]
            then
                INSTALL_DIR=$1
                shift
            fi
            ;;
        *)
            script_error "Invalid command or option: $C"
            usage 1
            ;;
    esac
done

if [[ -z $CMD ]]
then
    script_error "No command given"
    usage 1
fi

msg_info "Script directory: $SCRIPT_DIR"
msg_info "Script log directory: $LOG_DIR"
msg_info "This build script is designed for SPDK $SPDK_VERSION"

# Validate SPDK dir.
if [[ -z "$SPDK_ROOT_DIR" ]]
then
    msg_warn "SPDK root directory is not specified, will try ./spdk"
    SPDK_ROOT_DIR="spdk"
fi

SPDK_ROOT_DIR=$(realpath "$SPDK_ROOT_DIR")
if ! pushd "$SPDK_ROOT_DIR" > /dev/null
then
    script_error "Cannot change into SPDK directory $SPDK_ROOT_DIR"
    exit 1
fi
silent_popd
msg_info "SPDK directory: $SPDK_ROOT_DIR"

# Validate build type.
case $BUILD_TYPE in
    "debug" | "release")
        ;;
    *)
        script_error "Bad build type: $BUILD_TYPE"
        usage 1
        ;;
esac

# Validate target platform.
case $TARGET_PLATFORM in
    "x86_64-unknown-linux-gnu")
        TARGET_PLATFORM_x86_64="yes"
        ;;
    "aarch64-unknown-linux-gnu")
        ;;
    *)
        script_error "Bad target platform: $TARGET_PLATFORM"
        usage 1
        ;;
esac

EXT_TOOL="$SCRIPT_DIR/tool_$CMD.sh"
if [[ -f "$EXT_TOOL" ]]
then
    msg_info "Running tool '$CMD' ..."

    $EXT_TOOL
    R=$?

    if [[ $R -eq 0 ]]
    then
        msg_info "Running tool '$CMD': ok"
    else
        msg_error "Running tool '$CMD': failed with code $R"
    fi

    exit $R
else
    CMD="cmd_$CMD"
    $CMD $@
    exit $?
fi
