// Bound both wire data and expanded data before request admission or Chat access.
export const DESKTOP_WIRE_LIMIT=16*1024*1024;
export const DESKTOP_DECODED_LIMIT=32*1024*1024;
export class DesktopBodyError extends Error{
 constructor(code,status=400){super(code);this.status=status;}
}
function limited(stream,limit,code){
 let bytes=0;
 return stream.pipeThrough(new TransformStream({transform(chunk,controller){
  bytes+=chunk.byteLength;
  if(bytes>limit)throw new DesktopBodyError(code,413);
  controller.enqueue(chunk);
 }}));
}
export async function readDesktopRequest(req,{wireLimit=DESKTOP_WIRE_LIMIT,decodedLimit=DESKTOP_DECODED_LIMIT}={}){
 const encoding=(req.headers.get('content-encoding')||'identity').trim().toLowerCase()||'identity';
 if(!['identity','zstd'].includes(encoding))throw new DesktopBodyError('desktop_encoding_unsupported',415);
 const length=req.headers.get('content-length');
 if(length!==null&&(!/^\d+$/.test(length)||!Number.isSafeInteger(Number(length))))throw new DesktopBodyError('invalid_desktop_length');
 if(length!==null&&Number(length)>wireLimit)throw new DesktopBodyError('desktop_request_too_large',413);
 const wire=limited(req.body||new ReadableStream({start(c){c.close();}}),wireLimit,'desktop_request_too_large');
 let stream=wire;
 if(encoding==='zstd')stream=wire.pipeThrough(new DecompressionStream('zstd'));
 const reader=limited(stream,decodedLimit,'desktop_decoded_request_too_large').getReader();
 const decoder=new TextDecoder('utf-8',{fatal:true});let text='';
 try{
  while(true){
   let part;try{part=await reader.read();}catch(e){
    if(e instanceof DesktopBodyError)throw e;
    throw new DesktopBodyError(encoding==='zstd'?'invalid_desktop_compression':'desktop_body_read_failed');
   }
   if(part.done)break;
   try{text+=decoder.decode(part.value,{stream:true});}catch{throw new DesktopBodyError('invalid_desktop_utf8');}
  }
  try{text+=decoder.decode();}catch{throw new DesktopBodyError('invalid_desktop_utf8');}
 }finally{await reader.cancel().catch(()=>{});reader.releaseLock();}
 try{return JSON.parse(text);}catch{throw new DesktopBodyError('invalid_desktop_json');}
}
