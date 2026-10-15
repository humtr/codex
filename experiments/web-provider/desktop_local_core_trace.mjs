// Private synthetic-trial evidence, not a production transcript or executor.
// Preserve the owned public messages so a projection defect can be fixed/replayed
// without spending another Chat generation. Never record private reasoning.
import {writeFileSync} from 'node:fs';
import {DesktopToolSession} from './desktop_tool_session.mjs';
import {DesktopLocalTransport} from './desktop_local_transport.mjs';
const [bridge,session,root]=process.argv.slice(2),messages=new Map(),observations=new Map();let tools=[];
const text=DesktopToolSession.prototype.text,exchange=DesktopLocalTransport.prototype.exchange;
DesktopLocalTransport.prototype.exchange=function(payload,...args){tools=payload.tools;return exchange.call(this,payload,...args);};
DesktopToolSession.prototype.text=function(message,emit){
 const hidden=message.metadata?.is_visually_hidden_from_conversation===true;
 const visible=message.author?.role==='assistant'&&
  (!hidden&&message.recipient==='all'&&[null,undefined,'commentary','final'].includes(message.channel)&&message.content?.content_type==='text'||message.recipient?.startsWith('local.')&&message.content?.content_type==='code');
 // Ignored messages get metadata only, never private analysis text.
 const observation={id:message.id,role:message.author?.role,recipient:message.recipient,channel:message.channel,
  contentType:message.content?.content_type,status:message.status,endTurn:message.end_turn,hidden,
  gatewayProjection:message.id?.startsWith('report:')===true};
 let changed=JSON.stringify(observations.get(message.id))!==JSON.stringify(observation);
 observations.set(message.id,observation);
 if(visible){
  const clean={id:message.id,author:{role:'assistant'},recipient:message.recipient,channel:message.channel,
   content:message.content,status:message.status,end_turn:message.end_turn};
  if(JSON.stringify(messages.get(message.id))!==JSON.stringify(clean)){messages.set(message.id,clean);changed=true;}
 }
 if(changed)writeFileSync(root+'/public-trace.json',JSON.stringify({syntheticTrial:true,tools,messages:[...messages.values()],observations:[...observations.values()]}),{mode:0o600});
 return text.call(this,message,emit);
};
process.env.CODEX_CHAT_RETURN_TRANSPORT='local';process.env.CODEX_CHAT_LOCAL_MAX_DISPATCHES='3';
process.argv=[process.argv[0],process.argv[1],bridge,session,root+'/server.json'];
await import('./desktop_responses_server.mjs');
