//! Explicit read-only inspection of a user-authorized, already running app.
//! No hooks, chats, tools, account changes or credential values are exported.
use codex_chatgpt_provider::cdp::{Client, Descriptor};

#[tokio::test]
#[ignore = "requires explicit authorized descriptor, expected identity and project name; metadata reads only"]
async fn project_sidebar_and_exact_resource_report_admission_flags_without_mutation() {
    let descriptor = Descriptor::read(std::path::Path::new(
        &std::env::var_os("CHATGPT_PROVIDER_DESCRIPTOR").unwrap(),
    ))
    .unwrap();
    let expected: serde_json::Value = serde_json::from_slice(
        &codex_chatgpt_provider::cdp::private_read(
            std::path::Path::new(&std::env::var_os("CODEX_PROVIDER_EXPECTED_IDENTITY").unwrap()),
            4096,
        )
        .unwrap(),
    )
    .unwrap();
    let name = serde_json::json!(std::env::var("CHATGPT_PROVIDER_PROJECT_NAME").unwrap());
    let client = Client::connect(&descriptor).await.unwrap();
    let app = client.app().await.unwrap();
    let result = app.evaluate(&format!(r#"(async()=>{{
      const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];if(!s)throw Error('existing_bridge_required');
      const expected={expected},name={name},check=()=>{{const a=s.scope.get(s.shared.xNt);if(a.authLoading||a.accountLoading||a.accountId!==expected.account||a.userId!==expected.user||a.accountStructure!==expected.structure)throw Error('account_changed');return a;}};
      const a=check(),user=a.accountStructure==='personal'?a.accountUserId?.split('__')[0]:a.accountUserId;
      const flags=v=>{{const g=v.gizmo;return {{ownerMatches:g.author?.user_id===user,privateShare:g.share_recipient==='private',firstParty:!!g.first_party_project_type,snorlax:!!g.gizmo_snorlax_type,hasSharingSubjects:v.hasSharingSubjects===true,sharingSubjectCount:g.sharing?.subjects?.length??0,projectOnlyMemory:g.memory_scope==='project_v2'}};}};
      const list=await s.wire.safeGet('/gizmos/snorlax/sidebar',{{parameters:{{query:{{conversations_per_gizmo:0,cursor:null,limit:20,owned_only:true}}}}}});check();
      const found=list.items.filter(v=>v.gizmo.gizmo.display.name===name),matches=[];
      for(const item of found){{const detail=await s.wire.safeGet('/gizmos/{{gizmo_id_or_short_url}}',{{parameters:{{path:{{gizmo_id_or_short_url:item.gizmo.gizmo.id}},query:{{include_file_limits:true}}}}}});check();matches.push({{sidebar:flags(item.gizmo),exact:flags(detail),sameId:detail.gizmo.id===item.gizmo.gizmo.id}});}}
      return {{namedMatches:found.length,matches,chatSubmissions:0,mutations:0}};
    }})()"#)).await.unwrap();
    println!("{result}");
    assert_eq!(result["namedMatches"], 1);
}

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
    let expected = std::env::var_os("CODEX_PROVIDER_EXPECTED_IDENTITY")
        .map(|path| {
            serde_json::from_slice::<serde_json::Value>(
                &codex_chatgpt_provider::cdp::private_read(std::path::Path::new(&path), 4096)
                    .unwrap(),
            )
            .unwrap()
        })
        .unwrap_or(serde_json::Value::Null);
    let script = r#"(async(expected)=>{
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
      const before={account:auth?.accountId,user:auth?.userId,structure:auth?.accountStructure};
      const projects=await shared.$wt.safeGet('/gizmos/snorlax/sidebar',{parameters:{query:{conversations_per_gizmo:0,cursor:null,limit:20,owned_only:true}}});
      const projectShape=Array.isArray(projects.items)&&projects.items.every(item=>
        typeof item.gizmo?.gizmo?.id==='string'&&typeof item.gizmo?.gizmo?.display?.name==='string');
      let detailShape=true;
      if(projectShape&&projects.items.length){
        const detail=await shared.$wt.safeGet('/gizmos/{gizmo_id_or_short_url}',{parameters:{path:{gizmo_id_or_short_url:projects.items[0].gizmo.gizmo.id},query:{include_file_limits:true}}});
        detailShape=detail.gizmo?.id===projects.items[0].gizmo.gizmo.id&&typeof detail.gizmo?.display?.name==='string';
      }
      const after=scope.get(shared.xNt);
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
        wire:typeof shared.$wt?.streamPost==='function',
        restartable:typeof client?.getRestartBlockers==='function'&&client.getRestartBlockers().length===0,
        projects:projectShape&&detailShape,
        expectedIdentity:expected==null||['account','user','structure'].every(k=>before[k]===expected[k]),
        sameIdentity:before.account===after?.accountId&&before.user===after?.userId&&before.structure===after?.accountStructure
      };
    })"#;
    let result = app
        .evaluate(&format!("{script}({expected})"))
        .await
        .unwrap();
    assert_eq!(
        result,
        serde_json::json!({"scope":true,"authenticated":true,"localFunctions":true,"identityExport":true,"stream":true,"localFunctionGate":true,"wire":true,"restartable":true,"projects":true,"sameIdentity":true,"expectedIdentity":true})
    );
    println!(
        "{}",
        serde_json::json!({"accepted":true,"readOnlyAuthenticatedApp":true,"identityValuesExported":false,"hooksInstalled":false,"chatSubmissions":0})
    );
    let actions = app.evaluate(r#"(()=>{const s=globalThis[Symbol.for('termux.codex.app.bridge.v1')];return {lanes:s?[...s.lanes.values()].map(l=>({streamActive:l.streamActive===true,stopped:l.stopRequested===true,dispatches:l.starts.size,userMessages:l.userMessages,toolMessages:l.toolMessages,actions:[...l.messages.values()].filter(m=>m.author?.role==='tool').map(m=>{const d=m.metadata||{},p=d.jit_plugin_data||d.elicitation_metadata||{},f=(p.plugin_data||p).from_server||{},e=d.codex_mcp_elicitation||f.body?.codex_mcp_elicitation||{};return {messageStatus:m.status,serverType:f.type,connector:f.body?.connector_name,computerUse:f.body?.connector_id==='computer-use',operation:f.body?.operation,actionNames:f.body?.actions?.map(a=>a.name),elicitationKind:e.kind,elicitationStatus:e.status,responsePresent:d.codex_mcp_elicitation_response!=null};})})):[]};})()"#).await.unwrap();
    println!("Host action observation (no text/arguments): {actions}");
}
