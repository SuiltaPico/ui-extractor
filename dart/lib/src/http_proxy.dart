import 'dart:io';

/// Resolves proxy for HTTP(S) downloads.
///
/// Order: `http_proxy` / `https_proxy` env vars, then Windows Internet Settings
/// registry (system proxy).
String resolveHttpProxy(Uri uri) {
  final fromEnv = HttpClient.findProxyFromEnvironment(uri);
  if (fromEnv != 'DIRECT') {
    return fromEnv;
  }
  final systemProxy = _readWindowsInternetProxy();
  if (systemProxy == null) {
    return 'DIRECT';
  }
  return _proxyDirectiveForUri(uri, systemProxy);
}

String? _cachedWindowsInternetProxy;
bool _windowsInternetProxyResolved = false;

String? _readWindowsInternetProxy() {
  if (!Platform.isWindows) {
    return null;
  }
  if (_windowsInternetProxyResolved) {
    return _cachedWindowsInternetProxy;
  }
  _windowsInternetProxyResolved = true;
  try {
    final enable = _readRegDword(
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      'ProxyEnable',
    );
    if (enable != 1) {
      return null;
    }
    final server = _readRegString(
      r'HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings',
      'ProxyServer',
    );
    if (server == null || server.isEmpty) {
      return null;
    }
    _cachedWindowsInternetProxy = server;
    return server;
  } catch (_) {
    return null;
  }
}

int? _readRegDword(String key, String valueName) {
  final result = Process.runSync('reg', [
    'query',
    key,
    '/v',
    valueName,
  ]);
  if (result.exitCode != 0) {
    return null;
  }
  final match = RegExp(
    r'0x[0-9a-fA-F]+|\d+',
  ).firstMatch(result.stdout.toString());
  if (match == null) {
    return null;
  }
  final raw = match.group(0)!;
  return raw.startsWith('0x')
      ? int.parse(raw.substring(2), radix: 16)
      : int.parse(raw);
}

String? _readRegString(String key, String valueName) {
  final result = Process.runSync('reg', [
    'query',
    key,
    '/v',
    valueName,
  ]);
  if (result.exitCode != 0) {
    return null;
  }
  final match = RegExp(
    r'REG_SZ\s+(.*)\s*$',
    multiLine: true,
  ).firstMatch(result.stdout.toString());
  return match?.group(1)?.trim();
}

String _proxyDirectiveForUri(Uri uri, String proxyServer) {
  if (!proxyServer.contains('=')) {
    return 'PROXY $proxyServer';
  }
  final scheme = uri.scheme.toLowerCase();
  for (final part in proxyServer.split(';')) {
    final eq = part.indexOf('=');
    if (eq <= 0) {
      continue;
    }
    final key = part.substring(0, eq).trim().toLowerCase();
    final value = part.substring(eq + 1).trim();
    if (key == scheme) {
      return 'PROXY $value';
    }
  }
  return 'DIRECT';
}
