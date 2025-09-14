#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const glob = require('glob');

// Find all TSX files in components
const files = glob.sync('src/components/**/*.tsx');

files.forEach(file => {
  let content = fs.readFileSync(file, 'utf8');
  let modified = false;
  
  // 1. Standardize React imports - use named imports, no default React import
  if (content.includes('import React')) {
    const oldImport = content.match(/import React[^;]+;/);
    if (oldImport) {
      // Extract what's being imported
      const hasHooks = oldImport[0].includes('useState') || oldImport[0].includes('useEffect') || 
                       oldImport[0].includes('useCallback') || oldImport[0].includes('useMemo') ||
                       oldImport[0].includes('useRef');
      
      if (hasHooks) {
        // Already has named imports, just remove React
        content = content.replace(/import React,\s*{/, 'import {');
      } else if (oldImport[0] === 'import React from \'react\';') {
        // Remove pure React import since React 17+ doesn't need it
        content = content.replace(/import React from ['"]react['"];?\n?/, '');
      }
      modified = true;
    }
  }
  
  // 2. Convert React.FC to standard function components
  if (content.includes('React.FC')) {
    content = content.replace(/:\s*React\.FC<([^>]+)>\s*=/g, ' = ');
    content = content.replace(/:\s*React\.FC\s*=/g, ' = ');
    
    // Add proper typing to props in function signature
    const componentMatch = content.match(/export const (\w+) = \(([^)]*)\)/g);
    if (componentMatch) {
      componentMatch.forEach(match => {
        if (!match.includes(':')) {
          // Add typing if missing
          const name = match.match(/export const (\w+)/)[1];
          const propsType = `${name}Props`;
          if (content.includes(`interface ${propsType}`)) {
            content = content.replace(match, match.replace('()', `({ }: ${propsType})`));
          }
        }
      });
    }
    modified = true;
  }
  
  // 3. Standardize exports - use named exports with memo for performance
  const componentName = path.basename(file, '.tsx');
  
  // Check if it's a default export
  if (content.includes('export default')) {
    // Convert default export to named export with memo
    const defaultExportMatch = content.match(/export default (\w+);?/);
    if (defaultExportMatch) {
      const exportedName = defaultExportMatch[1];
      
      // Check if memo is imported
      if (!content.includes('import { memo')) {
        // Add memo to imports
        if (content.includes('import {')) {
          content = content.replace(/import {([^}]+)} from ['"]react['"]/, 'import { memo,$1} from \'react\'');
        } else {
          // Add new import line after first import
          const firstImport = content.indexOf('import');
          const endOfFirstImport = content.indexOf('\n', firstImport);
          content = content.slice(0, endOfFirstImport + 1) + 
                   'import { memo } from \'react\';\n' + 
                   content.slice(endOfFirstImport + 1);
        }
      }
      
      // Replace default export with named memo export
      content = content.replace(/export default (\w+);?/, `export const ${exportedName} = memo($1);`);
      modified = true;
    }
    
    // Handle inline default function exports
    const inlineMatch = content.match(/export default function (\w+)/);
    if (inlineMatch) {
      const funcName = inlineMatch[1];
      
      // Add memo import if needed
      if (!content.includes('import { memo')) {
        if (content.includes('import {')) {
          content = content.replace(/import {([^}]+)} from ['"]react['"]/, 'import { memo,$1} from \'react\'');
        } else {
          const firstImport = content.indexOf('import');
          const endOfFirstImport = content.indexOf('\n', firstImport);
          content = content.slice(0, endOfFirstImport + 1) + 
                   'import { memo } from \'react\';\n' + 
                   content.slice(endOfFirstImport + 1);
        }
      }
      
      // Convert to const with memo
      content = content.replace(/export default function (\w+)/, 'const $1');
      
      // Add export at the end
      if (!content.includes(`export const ${funcName} = memo`)) {
        content = content.trimEnd() + `\n\nexport const ${funcName} = memo(${funcName});\n`;
      }
      modified = true;
    }
  }
  
  // 4. Add memo to components that don't have it (for React.FC components)
  const namedExportMatch = content.match(/export const (\w+)\s*=\s*\(/);
  if (namedExportMatch && !content.includes('memo(')) {
    const compName = namedExportMatch[1];
    
    // Add memo import
    if (!content.includes('import { memo')) {
      if (content.includes('import {')) {
        content = content.replace(/import {([^}]+)} from ['"]react['"]/, 'import { memo,$1} from \'react\'');
      } else {
        const firstImport = content.indexOf('import');
        if (firstImport > -1) {
          const endOfFirstImport = content.indexOf('\n', firstImport);
          content = content.slice(0, endOfFirstImport + 1) + 
                   'import { memo } from \'react\';\n' + 
                   content.slice(endOfFirstImport + 1);
        }
      }
    }
    
    // Wrap component with memo
    const componentRegex = new RegExp(`export const ${compName}\\s*=\\s*\\(`);
    content = content.replace(componentRegex, `export const ${compName} = memo((`);
    
    // Find the end of the component and add closing paren
    const componentStart = content.indexOf(`export const ${compName}`);
    let braceCount = 0;
    let inString = false;
    let stringChar = '';
    let i = componentStart;
    
    // Skip to the opening of the function
    while (i < content.length && content[i] !== '{') i++;
    
    for (; i < content.length; i++) {
      const char = content[i];
      const prevChar = i > 0 ? content[i-1] : '';
      
      if (!inString) {
        if ((char === '"' || char === "'" || char === '`') && prevChar !== '\\') {
          inString = true;
          stringChar = char;
        } else if (char === '{') {
          braceCount++;
        } else if (char === '}') {
          braceCount--;
          if (braceCount === 0) {
            // Found the end of the component
            content = content.slice(0, i + 1) + ')' + content.slice(i + 1);
            break;
          }
        }
      } else {
        if (char === stringChar && prevChar !== '\\') {
          inString = false;
        }
      }
    }
    modified = true;
  }
  
  // 5. Clean up double semicolons
  content = content.replace(/;;/g, ';');
  
  // 6. Remove trailing whitespace
  content = content.split('\n').map(line => line.trimEnd()).join('\n');
  
  if (modified) {
    fs.writeFileSync(file, content);
    console.log(`✅ Standardized: ${file}`);
  }
});

console.log(`\n📋 Processed ${files.length} components`);