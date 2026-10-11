# JV Patch fixture

`80to1080-cut.syx` is four JV-1080 Patches cut from the `80TO1080` bank on
Planet Groove: User Patches 002 `AwesomeChoir`, 007 `Bright Lead`, 041
`Softpad` and 045 `Winds Of Chg`. Each is the 5 DT1 messages (Common and
Tones 1–4) at `11 pp xx xx` of `80TO1080.SYX`, copied byte for byte and in
file order (2,572 bytes).

- Original download: `syx_80to1080.zip` from
  <http://www.planet-groove.com/roland/patches.html> (file `80TO1080.SYX`,
  82,304 bytes, SHA-1 `f78a437bdcab24eff9a239263d705d9f279b412e`). Corpus
  entry `planet-groove`, see
  [Collect a Foreign patch corpus outside the repo](https://github.com/sigma/integral/issues/197).
- They cover an SR-JV80-04 Wave, a PCM-card Wave, all four Matrix Controls
  in use, a REVERSE switch, the Velocity Range Switch and a KEY-INTERVAL
  delay. The bank holds JV-1080 Patches only, so the XP and JV-2080 Common
  lengths are byte-built in the tests.

## Licence

From the bank's entry on the download page:

> **File** 80TO1080.ZIP in .SYX, .SVD or .MID format
>
> **Author** Unknown
>
> **Description** This bank contains about 45 patches that were originally
> created for the Roland JV-80. All patches have been available in the
> public domain for some years now.
