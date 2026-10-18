//! Explicit read-only inspection of a user-authorized, already running app.
//! No hooks, chats, tools, account changes or credential values are exported.
use codex_chatgpt_provider::cdp::{Client, Descriptor};

#[tokio::test]
#[ignore = "requires explicit CHATGPT_PROVIDER_DESCRIPTOR for an authorized logged-in app"]
async fn authenticated_app_exposes_current_identity_and_local_function_capabilities() {
    let path = std::env::var_os("CHATGPT_PROVIDER_DESCRIPTOR")
        .expect("explicit owned-app descriptor required");
    let descriptor = Descriptor::read(std::path::Path::new(&path)).unwrap();
    // An occupied legacy relay rejects this connection rather than displacing
    // its existing observer. Never retry, stop it or start a replacement here.
    let client = Client::connect(&descriptor).await.unwrap();
    let app = client.app().await.unwrap();
    let result = app.evaluate(r#"(async()=>{
      let node=document.querySelector('[contenteditable=true]')||document.querySelector('button');
      while(node&&!Object.keys(node).some(k=>k.startsWith('__reactFiber')))node=node.parentElement;
      let fiber=node?.[Object.keys(node).find(k=>k.startsWith('__reactFiber'))],scope;
      for(let i=0;fiber&&i<400&&!scope;i++,fiber=fiber.return){
        for(let hook=fiber.memoizedState,j=0;hook&&j<300;hook=hook.next,j++){
          const value=hook.memoizedState?.current;
          if(typeof value?.get==='function'&&typeof value?.set==='function'&&value.node&&value.chain&&value.query){scope=value;break;}
        }
      }
      if(!scope)return {scope:false};
      const api=await import('app://-/assets/app-initial-ed5807835d97.js');
      const shared=await import('app://-/assets/app-shared-2d77298f0100.js');
      const auth=scope.get(shared.xNt),client=scope.get(api.oMn);
      return {
        scope:true,
        authenticated:!!auth&&!auth.authLoading&&!auth.accountLoading&&auth.authMethod==='chatgpt'&&
          typeof auth.accountId==='string'&&!!auth.accountId&&auth.accountId===auth.authenticatedAccountId&&
          typeof auth.userId==='string'&&!!auth.userId&&auth.userId===auth.authenticatedUserId&&
          ['personal','workspace'].includes(auth.accountStructure),
        localFunctions:['aCn','bRt','yRt','eHt'].every(k=>typeof api[k]==='function'),
        identityExport:!!shared.xNt,
        stream:['prepareCompletionStream','startCompletionStream'].every(k=>typeof client?.[k]==='function'),
        localFunctionGate:scope.get(shared.HAt)===true,
        wire:typeof shared.$wt?.streamPost==='function'
      };
    })()"#).await.unwrap();
    assert_eq!(
        result,
        serde_json::json!({"scope":true,"authenticated":true,"localFunctions":true,"identityExport":true,"stream":true,"localFunctionGate":true,"wire":true})
    );
    println!(
        "{}",
        serde_json::json!({"accepted":true,"readOnlyAuthenticatedApp":true,"identityValuesExported":false,"hooksInstalled":false,"chatSubmissions":0})
    );
}
