
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wimplicit-function-declaration"

#include "spdk_helper.h"
#include <spdk/thread.h>

#pragma GCC diagnostic pop

void *spdk_rs_io_channel_get_ctx(struct spdk_io_channel *ch)
{
	return spdk_io_channel_get_ctx(ch);
}
