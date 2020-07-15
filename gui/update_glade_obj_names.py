#!/usr/bin/env python3
#
# A simple script to process a .glade-file and set GTK object names from Glade ids.
#

import xml.etree.ElementTree as ET
from getopt import getopt
import sys

DRY_RUN = False
IGNORE = [
    'GtkWindow',
    'GtkAdjustment'
]

def find_name(node):
    props = node.findall('property')
    names = [p for p in props if p.attrib.get('name') == 'name']
    assert len(names) <= 1
    if names:
        p = names[0]
        return p.text, p
    else:
        return None, None

def process(tree):
    for node in tree:
        if node.tag == 'object':
            klass = node.attrib.get('class')
            id = node.attrib.get('id')
            name, name_element = find_name(node)
            if id or name:
                print(' '.join(filter(lambda x: x is not None, [
                    klass,
                    f'id="{id}"' if id else None,
                    f'name="{name}"' if name else None,
                    '...'
                ])), end='')

                if klass in IGNORE or not id:
                    print(' (ignored)', end='')
                else:
                    if name == id:
                        print(' (ok)', end='')
                    elif name_element is not None:
                        name_element.text = id
                        print(' updated!', end='')
                    else:
                        name_element = ET.SubElement(node, 'property')
                        name_element.attrib['name'] = 'name'
                        name_element.text = id
                        print(' added!', end='')

                print('')

            for child in node: process(child)


opts,args = getopt(sys.argv[1:], 'n')
opt_names = [x[0] for x in opts]
if '-n' in opt_names: DRY_RUN = True
if len(args) != 1:
    print(f'usage: {sys.argv[0]} [-n] <file.glade>')
    sys.exit(1)

name = args[0]
file = open(name).read()
xml = ET.fromstring(file)

process(xml)

if not DRY_RUN:
    with open(name, 'w') as file:
        file.write(ET.tostring(xml, encoding='unicode', xml_declaration=True))
