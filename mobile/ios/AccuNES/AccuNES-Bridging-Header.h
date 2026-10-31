#ifndef AccuNES_Bridging_Header_h
#define AccuNES_Bridging_Header_h

#if __has_include("accunes.h")
#include "accunes.h"
#else
#include "../../../crates/accunes_ffi/include/accunes.h"
#endif

#endif /* AccuNES_Bridging_Header_h */
