import {mkdirSync,writeFileSync} from 'node:fs';
import {join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
const [candidate,gui]=process.argv.slice(2);
if(process.platform!=='android'||![candidate,gui].every(value=>value&&isAbsolute(value)))throw Error('Native Android and owned absolute roots required');
const {Client}=await import(pathToFileURL(join(candidate,'node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js')).href);
const {StdioClientTransport}=await import(pathToFileURL(join(candidate,'node_modules/@modelcontextprotocol/sdk/dist/esm/client/stdio.js')).href);
const root=join(gui,'mcp-fixture');mkdirSync(root,{recursive:true,mode:0o700});
const transport=new StdioClientTransport({command:process.execPath,args:[join(candidate,'src/cli.ts'),'mcp','--contract','native','--broker-socket',root+'/broker.sock'],env:{HOME:root,PATH:process.env.PATH,PREFIX:process.env.PREFIX,TMPDIR:process.env.TMPDIR,CODEX_CHATGPT_WEB_HOME:root},stderr:'pipe'});
const client=new Client({name:'owned-termux-catalog-probe',version:'1'},{capabilities:{}});
let stderrBytes=0;transport.stderr?.on('data',data=>stderrBytes+=data.length);
try{
 await client.connect(transport,{timeout:15000});
 const listing=await client.listTools();
 if(listing.tools.length<3||new Set(listing.tools.map(tool=>tool.name)).size!==listing.tools.length)throw Error('Native MCP catalog failed');
 const report={actualNativeStdioConnected:true,toolCount:listing.tools.length,toolNames:listing.tools.map(tool=>tool.name),schemasValid:listing.tools.every(tool=>tool.inputSchema.type==='object'),realToolExecutions:0,stderrBytes};
 if(!report.schemasValid)throw Error('MCP schemas not object-shaped');
 writeFileSync(join(gui,'mcp-catalog-proof.json'),JSON.stringify(report),{mode:0o600});console.log(JSON.stringify(report));
}finally{await client.close();}
