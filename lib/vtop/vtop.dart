/// The parts of the Rust bindings the rest of the app may use directly.
/// Everything else about VTOP goes through `vtop/` (pages, sign-in, the
/// server link); only `vtop/` imports `lib/src/` beyond the data types.
library;

export 'package:vitapmate/src/api/vtop/vtop_client.dart' show VtopClient;
export 'package:vitapmate/src/api/vtop_get_client.dart'
    show exportSessionState, fetchSemesters, vtopClientRegistrationNumber;
