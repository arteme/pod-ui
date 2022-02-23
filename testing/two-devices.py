# Simulate multiple devices answering to the device inquiry message,
# first of them not being a POD at all
from mididings import *
def hex(x): return bytes(bytearray.fromhex(x))

not_a_pod = SysEx(hex('F0 7E 7F 06 02 00 20 08 63 0E 50 02 20 31 32 35 F7'))
a_pod = SysEx(hex('F0 7E 7F 06 02 00 01 0C 00 00 00 03 30 32 33 30 F7'))


run(
    SysExFilter(hex('f0 7e 7f 06 01 f7')) >> [ not_a_pod, a_pod ]
)
