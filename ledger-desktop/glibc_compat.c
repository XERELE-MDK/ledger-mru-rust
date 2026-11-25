/* Makes the release binary run on older Linux distributions.
 *
 * Built on Arch (glibc 2.44), the binary asked for acosf/atan2f@GLIBC_2.43 and
 * hypotf@GLIBC_2.35, so it refused to start on Ubuntu 24.04 (2.39), Debian 13
 * (2.41) or Fedora 42 (2.41). These wrappers bind the calls to the original
 * x86_64 versions (GLIBC_2.2.5), present in every glibc. build.rs links this
 * file with -Wl,--wrap=acosf,... so every call to acosf goes to __wrap_acosf.
 *
 * pidfd_spawnp/pidfd_getpid (GLIBC_2.39) are referenced by Rust's std but only
 * used with the unstable Command::create_pidfd(), which this app never calls.
 * The stubs answer ENOSYS ("not available"), which std already handles.
 *
 * Result: needs glibc >= 2.34 (Ubuntu 22.04, Debian 12, Fedora 35, RHEL 9).
 * Check with: objdump -T target/release/ledger-desktop | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1
 */
#include <errno.h>
#include <sys/types.h>

__asm__(".symver acosf_v1, acosf@GLIBC_2.2.5");
__asm__(".symver atan2f_v1, atan2f@GLIBC_2.2.5");
__asm__(".symver hypotf_v1, hypotf@GLIBC_2.2.5");

float acosf_v1(float);
float atan2f_v1(float, float);
float hypotf_v1(float, float);

float __wrap_acosf(float x) { return acosf_v1(x); }
float __wrap_atan2f(float y, float x) { return atan2f_v1(y, x); }
float __wrap_hypotf(float x, float y) { return hypotf_v1(x, y); }

int __wrap_pidfd_spawnp(void) { return ENOSYS; }
pid_t __wrap_pidfd_getpid(int fd) { (void)fd; errno = ENOSYS; return -1; }
