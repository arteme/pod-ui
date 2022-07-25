# flex-specific: https://github.com/alexandrevicenzi/Flex/wiki/Custom-Settings

AUTHOR = 'arteme'
SITENAME = 'pod-ui'
SITEURL = ''

PATH = 'content'

TIMEZONE = 'Europe/Helsinki'

DEFAULT_LANG = 'en'

# Feed generation is usually not desired when developing
FEED_ALL_ATOM = None
CATEGORY_FEED_ATOM = None
TRANSLATION_FEED_ATOM = None
AUTHOR_FEED_ATOM = None
AUTHOR_FEED_RSS = None

# Blogroll
LINKS = (('github', 'https://github.com/arteme/pod-ui'),)

## Social widget
#SOCIAL = (('You can add links in your config file', '#'),
#          ('Another social link', '#'),)

DEFAULT_PAGINATION = False

# Uncomment following line if you want document-relative URLs when developing
#RELATIVE_URLS = True

THEME = 'theme'

INDEX_SAVE_AS = 'blog.html'

# config for the Flex theme
SITETITLE = 'pod-ui'
SITESUBTITLE = 'A modern cross-platform UI for controlling Line 6 POD family of modelling amp via MIDI'

SITELOGO = 'https://arteme.github.io/pod-ui/images/icon-small.png'

GITHUB_CORNER_URL = 'https://github.com/arteme/pod-ui'

STATIC_PATHS = [ 'images', 'static' ]
CUSTOM_CSS = 'static/extra.css'

MARKDOWN = {
    'extensions': [
        'markdown_checklist.extension'
    ]
}

from datetime import datetime
COPYRIGHT_NAME = '<a href="https://github.com/arteme">Artem E</a>'
COPYRIGHT_YEAR = datetime.now().year
