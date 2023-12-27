# Inofficial Documentation for Windows Night Light Registry Values

Night Light is controlled by two binary registry values: the state and the settings registry value - called by the salient words in their registry paths. There are other registry values associated with the feature, but they don't contain much information and their exact purpose and usefulness is unclear.

Writing one of the two registry values immediately applies the respective configuration. By some configurations, the underlying engine can be brought into a broken state that may persist until logging off or even restarting (in bad cases, the registry values need to be deleted first).

Both registry values have a prologue of identical structure that can also be found at the start of other values in the cloud store registry section (there are also variations of the prologue structure).

The following information isn't fully exhaustive and consists of some interpretation.

## Settings Registry Value

- Key path: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.settings\windows.data.bluelightreduction.settings`
- Value name: `Data`

This is the more complex and insightful registry value, which is why it's presented first. It can be used to adjust the schedule and night time color temperature as well as for informational purposes.

Registry values from different machines and from different times (one per line):

- `--` = non-existent byte
- `xx` = redacted

```
43 42 01 00 0a 02 01 00 2a 06 9e eb c3 84 06 2a 2b 0e 24 43 42 01|00 -- -- c2 0a 00 ca 14 0e 16 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 be 22 ca 32 0e 10 2e 36 00 ca 3c 0e 07 2e 37 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a0 e4 c6 a9 06 2a 2b 0e 21 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 ba 27 ca 32 0e 13 2e 11 00 ca 3c 0e 06 2e 26 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 d7 e6 c6 a9 06 2a 2b 0e 21 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 d0 26 ca 32 0e 13 2e 11 00 ca 3c 0e 06 2e 26 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 bf 89 c7 a9 06 2a 2b 0e 21 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 e6 25 ca 32 0e 13 2e 11 00 ca 3c 0e 06 2e 26 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 8a b1 b9 fb 05 2a 2b 0e 21 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 d6 39 ca 32 0e 13 2e 25 00 ca 3c 0e 07 2e 05 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 f8 88 c8 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 e6 25 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a2 ba c7 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 0a -- -- 00 ca 1e 0e 14 2e 1e 00 cf 28 e6 25 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 b4 ba c7 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 0a -- -- 00 ca 1e 0e 14 2e 1e 00 cf 28 e6 25 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c1 99 94 8d 06 2a 2b 0e 19 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 ba 27 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c1 98 99 fb 05 2a 2b 0e 1f 43 42 01|00 02 01 -- -- -- ca 14 0e 13 2e 39 00 ca 1e 0e 06 2e 39 00 cf 28 9e 4a ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 84 81 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 c8 65 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 9a 81 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 e0 12 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 9c 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 e0 12 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a2 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 cc 2b ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a8 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 c2 52 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 b8 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 c8 65 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 be 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c0 81 d2 a9 06 2a 2b 0e 1f 43 42 01|00 02 01 -- -- -- ca 14 0e 14 2e 0f 00 ca 1e 0e 09 2e 1e 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c2 81 d2 a9 06 2a 2b 0e 1f 43 42 01|00 02 01 -- -- -- ca 14 0e 14 2e 0f 00 ca 1e 0e 09 2e 2d 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c4 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 -- -- -- -- -- ca 14 0e 14 2e 0f 00 ca 1e 0e 09 2e 2d 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c6 81 d2 a9 06 2a 2b 0e 1f 43 42 01|00 02 01 -- -- -- ca 14 0e 14 2e 0f 00 ca 1e 0e 09 2e 2d 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c8 81 d2 a9 06 2a 2b 0e 1f 43 42 01|00 02 01 -- -- -- ca 14 0e 15 2e 0f 00 ca 1e 0e 09 2e 2d 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c9 cd d0 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 cc 2b ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ca 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 cc 81 d2 a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ce 81 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 a0 2d ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 d0 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 f0 33 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 d6 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 c0 3a ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 dc 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 90 41 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 dc 81 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 ba 27 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e2 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 f6 46 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e8 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 b0 4e ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ee 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 ea 55 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 f4 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 fc 59 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 fc 80 d2 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 09 2e 1e 00 cf 28 f4 63 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 d6 d7 d7 a9 06 2a 2b 0e 1b 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 cc 2b ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 8b ff 96 eb 05 2a 2b 0e 21 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 f0 29 ca 32 0e 15 2e 03 00 ca 3c 0e 06 2e 14 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 eb fe 96 eb 05 2a 2b 0e 23 43 42 01|00 02 01 -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 cf 28 f0 29 ca 32 0e 15 2e 03 00 ca 3c 0e 06 2e 14 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e8 da fb f9 05 2a 2b 0e 15 43 42 01|00 -- -- -- -- -- ca 14 0e 15 -- -- 00 ca 1e 0e 07 -- -- 00 -- -- -- -- ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 81 99 db a9 06 2a 2b 0e 1b 43 42 01|00 02 01 -- -- -- ca 14 -- -- -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 8e 29 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 91 99 db a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 -- -- 2e 0f 00 ca 1e 0e 07 2e 1e 00 cf 28 8e 29 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 d4 a6 db a9 06 2a 2b 0e 19 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e -- -- -- -- 00 cf 28 8e 29 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 dc a6 db a9 06 2a 2b 0e 1b 43 42 01|00 02 01 -- -- -- ca 14 0e 14 -- -- 00 ca 1e -- -- 2e 0f 00 cf 28 8e 29 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 fc 8d e9 a9 06 2a 2b 0e 1e 43 42 01|00 -- -- -- -- -- ca 14 0e 14 -- -- 00 ca 1e 0e 07 2e 1e 00 cf 28 f8 29 ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 c2 46 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ce c8 ee a9 06 2a 2b 0e 1d 43 42 01|00 02 01 -- -- -- ca 14 0e 08 2e 0f 00 ca 1e 0e 0e -- -- 00 cf 28 e2 2a ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 fe cf ee a9 06 2a 2b 0e 20 43 42 01|00 02 01 c2 0a 00 ca 14 0e 08 2e 0f 00 ca 1e 0e 0e -- -- 00 cf 28 e2 2a ca 32 -- -- -- -- 00 ca 3c -- -- -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 cd 8e db aa 06 2a 2b 0e 24 43 42 01|00 02 01 c2 0a 00 ca 14 -- -- -- -- 00 ca 1e 0e 03 2e 1e 00 cf 28 d0 26 ca 32 0e xx 2e xx 00 ca 3c 0e xx -- -- 00 -- -- -- 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a0 b8 db aa 06 2a 2b 0e 20 43 42 01|00 02 01 c2 0a 00 ca 14 -- -- -- -- 00 ca 1e -- -- -- -- 00 cf 28 e6 25 ca 32 0e xx 2e xx 00 ca 3c 0e xx -- -- 00 -- -- -- 00 00 00 00
^^^^^ ^^ ^^ ^^ ^^^^^ ^^ ^^^^^ |||||||||||||| ^^^^^ || || ^^^^^ ^^|^^ || || ||||| || ^^^^^ || || || || ^^ ^^^^^ || || || || ^^ ||||| ||||| ^^^^^ || || || || ^^ ^^^^^ || || || || ^^       || ^^^^^^^^^^^- Always present
                        ^^^^^ |||||||||||||| ^^^^^ || ||         |   || || ^^^^^ || ^^^^^ || || || ||    ^^^^^ || || || ||    ^^^^^ ||||| ^^^^^ || || || ||    ^^^^^ || || || ||    ^^^^^-||------------- Subsection signatures
                              ||||||||||||||       || ||         |   || || ||||| ||       || || || ||          || || || ||          |||||       || || || ||          || || || ||          ^^- Color temp preview active (boolean)
                              ||||||||||||||       || ||         |   || || ||||| ||       || || || ||          || || || ||          |||||       ^^ ^^ ^^ ^^          ^^ ^^ ^^ ^^- Sunset and sunrise times
                              ||||||||||||||       || ||         |   || || ||||| ||       || || || ||          || || || ||          ^^^^^- Night time color temperature
                              ||||||||||||||       || ||         |   || || ||||| ||       || || || ||          || || ^^ ^^- Explicit end minute (0-59, prefixed)
                              ||||||||||||||       || ||         |   || || ||||| ||       || || || ||          ^^ ^^- Explicit end hour (0-23, prefixed)
                              ||||||||||||||       || ||         |   || || ||||| ||       || || ^^ ^^- Explicit start minute (0-59, prefixed)
                              ||||||||||||||       || ||         |   || || ||||| ||       ^^ ^^- Explicit start hour (0-23, prefixed)
                              ||||||||||||||       || ||         |   || || ^^^^^ ^^- Explicit schedule chosen (signature, followed by zero-separator?)
                              ||||||||||||||       || ||         |   ^^ ^^- Schedule active (prefixed boolean?)
                              ||||||||||||||       || ||         ^- Data bytes to the right
                              ||||||||||||||       ^^ ^^- Number of data bytes (prefixed, VLQ-encoded)
                              ^^^^^^^^^^^^^^- Timestamp of change
```

- Timestamp of change: Unix epoch seconds, encoded as a VLQ ([variable-length quantity](https://en.wikipedia.org/wiki/Variable-length_quantity); little-endian order of 7-bit pieces). If this value is less than the stored one when writing a new registry value, writing will be immediately undone. Windows adds at least 2 seconds when changing a registry value, which can lead to an advancement past now when performing many changes.
- Schedule active: `02 01` if yes, otherwise nothing.
- Explicit schedule chosen: `c2 0a 00` if yes; otherwise nothing, meaning "Sunset to sunrise" is chosen. This is just the theoretical schedule type. Since explicit is (visually manifested in the GUI) used as the fallback for "Sunset to sunrise" when location services are turned off, the state must be checked to find the effective schedule type. Location services are turned on only if all values named `Value` of the following registry key paths exactly equal `Allow` (case-sensitive):
  - `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location`
  - `HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location`
  - `HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location\NonPackaged`
- Night time color temperature: In [Kelvin](https://en.wikipedia.org/wiki/Kelvin#Colour_temperature). [Zigzag](https://gist.github.com/mfuerstenau/ba870a29e16536fdbaba)- and then VLQ-encoded integer (zigzag is to make negative values positive, although there should only be positive values). If not present, a default chosen by Windows applies.
- Sunset and sunrise times: Same format as for explicit-schedule times. Was not available before an automatic transition for the author. Exact events of update are unknown, but one is when switching from explicit schedule to "Sunset to sunrise" with the official GUI.
- Color temp preview active: When using the official GUI: whether night time color temperature is being adjusted in this moment by dragging the slider. Setting this makes for an abrupt color temperature change as opposed to the otherwise smooth transitions. Shouldn't be changed or held active during changes to anything that may lead to a state change, because this can lead to inconsistent states of the underlying engine.
- Stray zero-bytes can't be understood as padding, and individual data values of 0 set via GUI don't make it into the binary data (see, e.g., schedule-active flag and time values). This could mean that zero-bytes are some kind of separator, in which case there would be multiple empty regions at the end. If zero-bytes have a special meaning in the binary data, it may even be essential to leave values out that are 0 when building the registry value yourself. Since the data bytes always start with a zero, it may be a prefix rather than a suffix; however, data like "Explicit schedule chosen", which includes a trailing zero, raises questions towards that hypothesis (maybe it's a zero-prefix with no data after it in the context of the subsection, like at the end of the registry value).

## State Registry Value

- Key path: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate`
- Value name: `Data`

This registry value can be used to manually toggle Night Light (as opposed to it being automatically toggled by a schedule) and for informational purposes.

```
43 42 01 00 0a 02 01 00 2a 06 bb b0 b9 fb 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 e3 ff b4 bd d9 ef a4 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 95 c1 b2 ed 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 f7 c7 9f e5 80 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 97 c1 b2 ed 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 b3 bf e5 e8 80 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a8 c1 b2 ed 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 87 91 e9 f7 80 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 aa c1 b2 ed 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 bc b6 bd fc 80 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 dc c5 b2 ed 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 be b3 a8 ae 98 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 de c5 b2 ed 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 91 d3 c6 b3 98 ee e1 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 8b ff 96 eb 05 2a 2b 0e 10 43 42 01|00 -- -- -- -- -- c6 14 94 99 a5 8a 8e a6 d7 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ea fe 96 eb 05 2a 2b 0e 12 43 42 01|00 10 00 -- -- -- c6 14 8b ba bb ed 8c a6 d7 ea 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a3 db fb f9 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 dc 85 df e0 a6 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e9 da fb f9 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 b6 e6 c8 d0 a4 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 b7 91 94 8d 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 ee d8 f3 8b b5 a8 f9 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 bb b0 b9 fb 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 e3 ff b4 bd d9 ef a4 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a3 db fb f9 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 dc 85 df e0 a6 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e9 da fb f9 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 b6 e6 c8 d0 a4 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a3 db fb f9 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 dc 85 df e0 a6 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e9 da fb f9 05 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 b6 e6 c8 d0 a4 e7 9d eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 bb b0 b9 fb 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 e3 ff b4 bd d9 ef a4 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 bb b0 b9 fb 05 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 e3 ff b4 bd d9 ef a4 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 9a eb c3 84 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 9f be ff 88 de 96 d0 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 b7 91 94 8d 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 ee d8 f3 8b b5 a8 f9 eb 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c2 cd 9a a6 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 86 e2 9e 8d 8a e2 f0 ec 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 9c ce 9a a6 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 f9 9f be bb 8d e2 f0 ec 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a5 81 c7 a9 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 8c ec ba b8 ef dc 80 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 cd 81 c7 a9 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 9d 9a f0 f9 f0 dc 80 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 86 90 c7 a9 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 8f 90 82 ea b5 dd 80 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 a2 ba c7 a9 06 2a 2b 0e 10 43 42 01|00 -- -- -- -- -- c6 14 fd fe f8 92 ff de 80 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 f8 88 c8 a9 06 2a 2b 0e 12 43 42 01|00 10 00 -- -- -- c6 14 a9 a5 92 a4 f6 e1 80 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c8 9e d0 a9 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 c4 c2 96 9b f0 88 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 c3 80 d2 a9 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 f0 f7 bb d3 a5 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 e7 80 d2 a9 06 2a 2b 0e 12 43 42 01|00 10 00 -- -- -- c6 14 81 e2 91 ff a6 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 84 81 d2 a9 06 2a 2b 0e 13 43 42 01|00 -- -- d0 0a 02 c6 14 85 a0 f2 be a7 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 86 81 d2 a9 06 2a 2b 0e 12 43 42 01|00 10 00 -- -- -- c6 14 bb b3 81 91 a8 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 92 81 d2 a9 06 2a 2b 0e 10 43 42 01|00 -- -- -- -- -- c6 14 ea de bb cd a8 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 94 81 d2 a9 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 e8 e3 fc d5 a8 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 aa 81 d2 a9 06 2a 2b 0e 10 43 42 01|00 -- -- -- -- -- c6 14 e5 a7 8b c1 a9 91 81 ed 01 00 00 00 00
43 42 01 00 0a 02 01 00 2a 06 ae 81 d2 a9 06 2a 2b 0e 15 43 42 01|00 10 00 d0 0a 02 c6 14 e6 fd 92 d6 a9 91 81 ed 01 00 00 00 00
||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||^^ || || ||||| || ^^^^^ |||||||||||||||||||||||||| ^^^^^^^^^^^- Always present
||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||   || || ^^^^^ || ^^^^^-||||||||||||||||||||||||||------------- Subsection signatures
||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||   || ||       ||       ^^^^^^^^^^^^^^^^^^^^^^^^^^- `FILETIME` structure
||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||   || ||       ^^- Manually transitioned (enum or bit field?)
||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||||   ^^ ^^- Active (prefix, enum or bit field, followed by zero-separator?)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^- See above
```

- Active: `10 00` if Night Light is currently active; otherwise nothing.
- Manually transitioned: Signature and `02` if yes; otherwise nothing, meaning transitioned by schedule (automatically time-based by underlying engine).
- [`FILETIME`](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime) structure: VLQ-encoded. The precise timestamp of change that doesn't suffer from the imperfection of the prologue timestamp.
