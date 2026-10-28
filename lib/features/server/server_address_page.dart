import 'package:flutter/widgets.dart';
import 'package:vitapmate/features/server/widgets/server_format.dart';
import 'package:vitapmate/features/server/widgets/server_url_section.dart';

/// The server this phone uses: change it, or go back to the default.
class ServerAddressPage extends StatelessWidget {
  const ServerAddressPage({super.key});

  @override
  Widget build(BuildContext context) => const ServerSubPage(
    title: 'Server address',
    children: [
      ServerUrlSection(),
      ServerNote(
        'Switching servers unlinks this phone from the old one: link it and set up OTP forwarding again on the new one.',
      ),
    ],
  );
}
