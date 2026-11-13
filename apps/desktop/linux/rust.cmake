# Builds the Rust bridge and stages it beside the Flutter executable.
#
# The bridge is a single cdylib (`libsosumi_bridge.so`) produced by Cargo. This
# module runs Cargo at build time and places the artifact where the rpath the
# runner sets (`$ORIGIN/lib`) will find it.
#
# Why not cargokit: flutter_rust_bridge's helper exists to handle many
# platforms, cross-compilation, and plugin layouts. Sosumi targets Linux only
# and already owns its workspace, so invoking Cargo directly is less machinery
# to debug and keeps the build visible.

set(RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
set(RUST_MANIFEST "${RUST_DIR}/Cargo.toml")
set(RUST_LIB_NAME "libsosumi_bridge.so")

# Release unless the Dart build was a debug build.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(RUST_PROFILE_DIR "debug")
    set(RUST_PROFILE_FLAG "")
else()
    set(RUST_PROFILE_DIR "release")
    set(RUST_PROFILE_FLAG "--release")
endif()

find_program(CARGO_EXECUTABLE cargo REQUIRED
    HINTS "$ENV{HOME}/.cargo/bin" /usr/local/bin /usr/bin)

message(STATUS "Sosumi: building Rust bridge with ${CARGO_EXECUTABLE} (profile ${RUST_PROFILE_DIR})")

set(RUST_TARGET_DIR "${RUST_DIR}/target/${RUST_PROFILE_DIR}")
set(RUST_LIB_PATH "${RUST_TARGET_DIR}/${RUST_LIB_NAME}")

# The runner loads libraries from `$ORIGIN/lib`, so the bridge is staged into
# the bundle's lib directory. Using the concrete path here rather than
# `$<TARGET_FILE_DIR:...>` matters: the generator expression would introduce a
# dependency from this target back onto the executable, and CMake rejects that
# cycle.
set(RUST_BUNDLE_DIR "${BUILD_BUNDLE_DIR}/lib")

# Re-run Cargo when any Rust source changes. A directory stamp is not precise,
# so the rule depends on the workspace manifest and the bridge sources, and
# Cargo itself decides what actually needs rebuilding.
file(GLOB_RECURSE RUST_SOURCES
    CONFIGURE_DEPENDS
    "${RUST_DIR}/crates/*/src/*.rs"
    "${RUST_DIR}/packages/*/src/*.rs"
)

add_custom_command(
    OUTPUT "${RUST_LIB_PATH}"
    COMMAND "${CARGO_EXECUTABLE}" build ${RUST_PROFILE_FLAG} -p sosumi-bridge
    WORKING_DIRECTORY "${RUST_DIR}"
    DEPENDS "${RUST_MANIFEST}" ${RUST_SOURCES}
    COMMENT "Building sosumi-bridge (cargo ${RUST_PROFILE_DIR})"
    VERBATIM
)

add_custom_target(sosumi_bridge ALL DEPENDS "${RUST_LIB_PATH}")

# Stage the library next to the executable so the runner finds it via rpath.
add_custom_command(
    TARGET sosumi_bridge POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${RUST_LIB_PATH}"
            "${RUST_BUNDLE_DIR}/${RUST_LIB_NAME}"
    COMMENT "Staging ${RUST_LIB_NAME}"
    VERBATIM
)
