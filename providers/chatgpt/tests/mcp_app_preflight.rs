//! Explicit authenticated-app preparation proof. Block both app client start and
//! conversation POST before inference; never export credentials or private text.
use codex_chatgpt_provider::{
    app::{Bridge, Remote},
    cdp::{private_read, Client, Descriptor},
    journal::random_id,
    protocol::Error,
};
use serde_json::{json, Value};
use std::path::PathBuf;
#[tokio::test]
#[ignore = "requires explicit CODEX_PROVIDER_MCP_DESCRIPTOR and expected private identity; no inference"]
async fn actual_app_builds_mcp_input_without_local_signatures_or_visible_environment_context() {
    let descriptor = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_MCP_DESCRIPTOR")
            .expect("explicit owned app descriptor required"),
    );
    let expected: Value = serde_json::from_slice(
        &private_read(
            &PathBuf::from(
                std::env::var_os("CODEX_PROVIDER_EXPECTED_IDENTITY")
                    .expect("expected account identity required"),
            ),
            4096,
        )
        .unwrap(),
    )
    .unwrap();
    let client = Client::connect(&Descriptor::read(&descriptor).unwrap())
        .await
        .unwrap();
    let app = client.app().await.unwrap();
    let released_settled = if std::env::var("CODEX_PROVIDER_RELEASE_SETTLED_FIXTURE").as_deref()
        == Ok("1")
    {
        let script = include_str!("../src/app_bridge.js");
        app.evaluate(&format!(r#"(async()=>{{const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];if(!s)return false;
          if(!(s.lanes instanceof Map)||typeof s.owner!=='string'||!s.owner||s.owner.length>256)throw Error('owned_fixture_required');
          const expected={expected},auth=s.scope.get(s.shared.xNt);
          if(s.lanes.size>0&&(auth?.accountId!==expected.account||auth?.userId!==expected.user||auth?.accountStructure!==expected.structure||auth.authLoading||auth.accountLoading))throw Error('fixture_account_changed');
          for(const k of ['prepareCompletionStream','startCompletionStream'])if(typeof s.saved?.[k]!=='function'||![s.saved[k],s.hooks?.[k]].includes(s.client?.[k]))throw Error('owned_fixture_hooks_required');
          if(typeof s.saved?.streamPost!=='function'||![s.saved.streamPost,s.hooks?.streamPost].includes(s.wire?.streamPost))throw Error('owned_fixture_hooks_required');
          for(const l of s.lanes.values()){{
            const messages=[...(l.messages?.values()||[])];
            if(l.pending||l.streamActive||JSON.stringify(l.terminals)!=='["done"]'||messages.some(m=>m.recipient==='local.'+l.signature?.name)||!messages.some(m=>m.author?.role==='assistant'&&(m.recipient==null||m.recipient==='all')&&m.channel==='final'&&m.status==='finished_successfully'&&m.end_turn===true))throw Error('settled_fixture_required');
          }}
          for(const lane of [...s.lanes.keys()])await ({script})({{op:'release',lane,owner:s.owner}});
          return (await ({script})({{op:'dispose',owner:s.owner}})).disposed===true;
        }})()"#)).await.unwrap()==true
    } else {
        false
    };
    let idle=app.evaluate("(()=>{const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];return {available:!s,activeLanes:s?.lanes?.size??0,observations:s?[...s.lanes.values()].map(l=>({pending:l.pending===true,streamActive:l.streamActive===true,stopped:l.stopRequested===true,hasRound:typeof l.round==='string',terminals:(l.terminals||[]).map(v=>['done','abort','aborted','cancelled','canceled'].includes(v)?v:'other'),nativeCalls:[...(l.messages?.values()||[])].filter(m=>m.recipient==='local.'+l.signature?.name).length,publicFinal:[...(l.messages?.values()||[])].some(m=>m.author?.role==='assistant'&&(m.recipient==null||m.recipient==='all')&&m.channel==='final'&&m.status==='finished_successfully'&&m.end_turn===true)})):[]};})()").await.unwrap();
    eprintln!("Existing bridge lifecycle observation: {idle}");
    assert_eq!(
        idle["available"], true,
        "existing owned bridge is preserved; do not replace its hooks"
    );
    // The private Chromium relay has one owner; multiplex the already verified
    // connection rather than trying to create a second relay peer.
    let remote = Remote::pipe(client.clone(), random_id().unwrap()).with_mcp();
    let identity = remote
        .operation(json!({"op":"identity","lane":"mcp-preflight"}))
        .await;
    if identity.is_err() {
        let shape=app.evaluate("(()=>{const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')],a=s.scope.get(s.shared.xNt);return {authLoading:a?.authLoading===true,accountLoading:a?.accountLoading===true,chatgptMethod:a?.authMethod==='chatgpt',accountPresent:!!a?.accountId,userPresent:!!a?.userId,accountCoherent:!!a?.accountId&&a.accountId===a.authenticatedAccountId,userCoherent:!!a?.userId&&a.userId===a.authenticatedUserId,structureSupported:['personal','workspace'].includes(a?.accountStructure)};})()").await.unwrap();
        eprintln!("Private app authentication admission flags: {shape}");
        remote.operation(json!({"op":"dispose"})).await.unwrap();
    }
    let identity = identity.unwrap();
    let same_account = ["account", "user", "structure"]
        .iter()
        .all(|k| identity[k] == expected[k]);
    if !same_account {
        remote.operation(json!({"op":"dispose"})).await.unwrap();
    }
    assert!(
        same_account,
        "authenticated account differs; no request authorized"
    );
    remote.operation(json!({"op":"open","lane":"mcp-preflight","signature":{"name":"unused_virtual_gateway"}})).await.unwrap();
    let guarded=app.evaluate(r#"(()=>{
      const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];
      const start=s.saved.startCompletionStream,post=s.wire.streamPost;
      const proof=s.preflight={starts:0,posts:0};
      proof.restore=()=>{if(s.saved.startCompletionStream===proof.start)s.saved.startCompletionStream=start;if(s.wire.streamPost===proof.post)s.wire.streamPost=post;delete s.preflight;};
      proof.start=function(options){
        const r=options?.request;proof.starts++;
        proof.localSignatures=r?.local_function_signatures?.length??0;
        proof.localNames=r?.local_function_names?.length??0;
        proof.users=r?.messages?.filter(m=>m.author?.role==='user').length;
        proof.hiddenEnvironment=r?.messages?.some(m=>m.author?.role==='user'&&m.content?.parts?.includes('<environment_context>owned synthetic fixture</environment_context>')&&m.metadata?.is_visually_hidden_from_conversation===true);
        proof.literalUserVisible=r?.messages?.some(m=>m.author?.role==='user'&&m.content?.parts?.includes('literal <environment_context /> request')&&!m.metadata?.is_visually_hidden_from_conversation);
        throw Error('owned_preflight_inference_blocked');
      };
      proof.post=function(path,...args){if(path==='/f/conversation'){proof.posts++;throw Error('owned_preflight_inference_blocked');}return post.call(this,path,...args);};
      s.saved.startCompletionStream=proof.start;s.wire.streamPost=proof.post;
      return {guarded:true,mode:s.mode};
    })()"#).await.unwrap();
    assert_eq!(guarded["mode"], "mcp");
    let result: Result<Value,Error>=async {
        remote.operation(json!({"op":"submit","lane":"mcp-preflight","round":"owned-preflight","model":"gpt-6","messages":[{"role":"user","text":"<environment_context>owned synthetic fixture</environment_context>","hidden":true},{"role":"user","text":"literal <environment_context /> request","hidden":false}],"instructions":["Synthetic request preparation only. No inference."],"results":[]})).await?;
        let proof=tokio::time::timeout(std::time::Duration::from_secs(20),async{loop{
            let proof=app.evaluate("(()=>{const p=globalThis[Symbol.for('termux.codex.app.bridge.v1')].preflight;return {starts:p.starts,posts:p.posts,localSignatures:p.localSignatures,localNames:p.localNames,users:p.users,hiddenEnvironment:p.hiddenEnvironment,literalUserVisible:p.literalUserVisible};})()").await?;
            if proof["starts"].as_u64().unwrap()>0||proof["posts"].as_u64().unwrap()>0{break Ok(proof);}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }}).await;
        // Return the proof for assertions only after owned hooks/lane cleanup.
        proof.map_err(|_|Error("owned_preflight_observation_timeout"))?
    }.await;
    let _ = remote
        .operation(json!({"op":"stop","lane":"mcp-preflight"}))
        .await;
    app.evaluate("(()=>{globalThis[Symbol.for('termux.codex.app.bridge.v1')]?.preflight?.restore();return true;})()").await.unwrap();
    remote
        .operation(json!({"op":"release","lane":"mcp-preflight"}))
        .await
        .unwrap();
    remote.operation(json!({"op":"dispose"})).await.unwrap();
    let proof = result.unwrap();
    assert_eq!(proof["starts"], 1);
    assert_eq!(proof["posts"], 0);
    assert_eq!(proof["localSignatures"], 0);
    assert_eq!(proof["localNames"], 0);
    assert_eq!(proof["users"], 2);
    assert_eq!(proof["hiddenEnvironment"], true);
    assert_eq!(proof["literalUserVisible"], true);
    println!(
        "{}",
        json!({"accepted":true,"actualSignedAppConstruction":true,"localFunctionSignatures":0,"conversationPosts":0,"chatSubmissions":0,"automaticContextHidden":true,"literalUserPreserved":true,"connectorAckQualified":false,"ownedHooksRemoved":true,"priorSettledFixtureReleased":released_settled})
    );
}
