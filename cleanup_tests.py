import re
import os

path = '/home/edohwares/Desktop/Room/grantfox/Revora-Contracts/src/test.rs'

with open(path, 'r') as f:
    content = f.read()

# Fix the mess: 'let issuer = let issuer = admin.clone();\n    .clone();'
# Pattern: let issuer = let issuer = ([a-z_]+)\.clone\(\);\n\s+\.clone\(\);
content = re.sub(r'let issuer = let issuer = ([a-z_]+)\.clone\(\);\n\s+\.clone\(\);', 
                 r'let issuer = \1.clone();', content)

# Also case without the newline mess
content = re.sub(r'let issuer = let issuer = ([a-z_]+)\.clone\(\);', 
                 r'let issuer = \1.clone();', content)

# Just in case there's more duplication
content = re.sub(r'let issuer = let issuer =', 'let issuer =', content)

# Fix get_offerings_page cursor
content = re.sub(r'\.get_offerings_page\(&issuer,\s*&symbol_short!\("def"\),\s*&c([0-9]+)\.unwrap\(\),\s*&([0-9]+)\)', 
                 r'.get_offerings_page(&issuer, &symbol_short!("def"), &c\1.unwrap(), &\2)', content)

# Final cleanup of double ampersands
content = content.replace('&&', '&')
content = content.replace('&&', '&')

with open(path, 'w') as f:
    f.write(content)
