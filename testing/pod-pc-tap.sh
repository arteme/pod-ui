#!/bin/bash
#
# A script to set up two way connection between a POD and a PC (via rtpmidid)
# with extra monitoring tap to aseqdump.
#
# - Connect the pod
# - Start avahi for rtpmidid: sudo systemctl start avahi-daemon
# - Start rtpmidid: ./build/src/rtpmidid
# - Start aseqdump
# - Check midi ports: aconnect -l / aseqdump -l
# - Run the tap
 
: ${POD:=24:0}
: ${PC=128:1}
: ${DUMP=129:0}

aconnect $POD $PC
aconnect $POD $DUMP

aconnect $PC $POD
aconnect $PC $DUMP
