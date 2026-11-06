# Session labels on Ripple

The desktop client sends its own name and version, plus conventional macOS and
CPU tokens in `User-Agent`. For example:

```text
RippleNative/0.4.0-beta.4 (Macintosh; ARM Mac OS X 27_2; aarch64)
```

The OS version is read from the Mac at runtime. No username, device name or
machine identifier is included. The client does not claim to run Safari or
Chrome. Account and media requests use this same client identity.

The website owns the Sessions table. A generic browser parser may not recognize
the new product name until the site adds a rule. Site owners can recognize
`RippleNative/<version>` before applying their usual browser fallback. Example
PHP adapter (map these fields to the site's existing session model):

```php
function rippleNativeAgent(string $ua): ?array {
    if (!preg_match('~^RippleNative/([0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.-]+)?)\s~', $ua, $client)) {
        return null;
    }
    $osVersion = null;
    if (preg_match('~Mac OS X ([0-9]+(?:_[0-9]+){1,2})~', $ua, $os)) {
        $osVersion = str_replace('_', '.', $os[1]);
    }
    return [
        'browser' => 'Ripple Native',
        'browser_version' => $client[1],
        'os' => str_contains($ua, 'Macintosh') ? 'macOS' : 'Unknown OS',
        'os_version' => $osVersion,
    ];
}
```

Older session rows may retain the user agent stored when they were created. The
client cannot rewrite those server records. IP geolocation is also determined
by the website, independently of this header.
