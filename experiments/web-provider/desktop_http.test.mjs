import {test,expect} from 'bun:test';
import {readDesktopRequest} from './desktop_http.mjs';
const request=(body,headers={})=>new Request('http://127.0.0.1/v1/responses',{method:'POST',body,headers});
test('identity and zstd preserve a complete request larger than the former 2 MiB cap',async()=>{
 const value={model:'native',stream:true,input:'한글 😀'.repeat(220000)};
 const body=new TextEncoder().encode(JSON.stringify(value));expect(body.length).toBeGreaterThan(2*1024*1024);
 expect(await readDesktopRequest(request(body))).toEqual(value);
 expect(await readDesktopRequest(request(Bun.zstdCompressSync(body),{'content-encoding':'zstd'}))).toEqual(value);
});
test('bounded decoding rejects wire and compressed expansion before parsing',async()=>{
 const body=new TextEncoder().encode(JSON.stringify({text:'x'.repeat(20000)})),packed=Bun.zstdCompressSync(body);
 await expect(readDesktopRequest(request(body),{wireLimit:100,decodedLimit:30000})).rejects.toThrow('desktop_request_too_large');
 await expect(readDesktopRequest(request(packed,{'content-encoding':'zstd'}),{wireLimit:1000,decodedLimit:100})).rejects.toThrow('desktop_decoded_request_too_large');
 await expect(readDesktopRequest(request('{}',{'content-length':'101'}),{wireLimit:100})).rejects.toThrow('desktop_request_too_large');
});
test('invalid encoding, compression, UTF-8 and JSON have distinct content-free errors',async()=>{
 for(const [body,headers,code] of [
  ['secret',{'content-encoding':'gzip'},'desktop_encoding_unsupported'],
  ['secret',{'content-encoding':'zstd'},'invalid_desktop_compression'],
  [new Uint8Array([34,255,34]),{},'invalid_desktop_utf8'],
  ['{"secret":',{},'invalid_desktop_json'],
  ['{}',{'content-length':'-1'},'invalid_desktop_length']
 ])await expect(readDesktopRequest(request(body,headers))).rejects.toThrow(code);
});
test('streamed split UTF-8 is decoded intact and oversized streams cancel upstream',async()=>{
 const body=new TextEncoder().encode('{"text":"한글😀"}');
 const chunks=new ReadableStream({start(c){for(const byte of body)c.enqueue(new Uint8Array([byte]));c.close();}});
 expect(await readDesktopRequest(request(chunks))).toEqual({text:'한글😀'});
 let cancelled=false,index=0;
 const endless=new ReadableStream({pull(c){c.enqueue(new Uint8Array([index++]));},cancel(){cancelled=true;}});
 await expect(readDesktopRequest(request(endless),{wireLimit:4})).rejects.toThrow('desktop_request_too_large');
 await new Promise(ok=>setTimeout(ok,10));expect(cancelled).toBe(true);
});
