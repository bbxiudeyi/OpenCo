use serde_json::{json, Value};

/// Execute a delegation: call an agent with a task and return the result.
/// This is called from within the tool-calling loop in chat_with_tools_sse.
pub fn tool_delegate(
    agent_name: &str,
    task: &str,
    org: &crate::config::OrgChart,
    agents: &std::collections::HashMap<String, crate::config::AgentConfig>,
    models: &[crate::config::ModelEntry],
    global_config: &crate::config::Config,
) -> String {
    // Find the target agent's position
    let position = match org.positions.iter().find(|p| p.agents.contains(&agent_name.to_string())) {
        Some(p) => p,
        None => return format!("未找到agent：{}。可用agent：{}", agent_name, available_agents(org)),
    };

    let agent = match agents.get(agent_name) {
        Some(a) => a.clone(),
        None => return format!("Agent {} 不存在。", agent_name),
    };

    // Build system prompt for this position
    let system_prompt = crate::org::build_system_prompt(org, position, &agent);

    // Determine which tools this position can use
    let subordinates = crate::org::get_subordinates(org, &position.id);
    let mut tool_names = agent.tools.clone();

    // Add delegate tool if this position has subordinates
    if !subordinates.is_empty() {
        tool_names.push("_delegate".to_string()); // marker, handled specially
    }

    // Build tools JSON
    let tools_json = if !subordinates.is_empty() {
        let sub_names: Vec<String> = subordinates.iter().flat_map(|s| s.agents.clone()).collect();
        let sub_display = if sub_names.is_empty() {
            subordinates.iter().map(|s| s.title.as_str()).collect::<Vec<_>>().join("、")
        } else {
            sub_names.join("、")
        };
        let mut tools_arr = crate::tools::get_tools_json(&agent.tools);
        let arr = tools_arr.as_array_mut().unwrap();
        arr.push(json!({
            "type": "function",
            "function": {
                "name": "delegate",
                "description": format!("将任务委派给下属。你的下属有：{}。", sub_display),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "agent_name": { "type": "string", "description": "下属agent名称" },
                        "task": { "type": "string", "description": "要委派的任务描述" }
                    },
                    "required": ["agent_name", "task"]
                }
            }
        }));
        Value::Array(arr.clone())
    } else {
        crate::tools::get_tools_json(&agent.tools)
    };

    // Resolve LLM params via global config
    let (api_url, api_key, model) = match global_config.resolve_model(agent.model_name.as_deref()) {
        Some(entry) => (entry.api_url.clone(), entry.api_key.clone(), entry.model.clone()),
        None => return "未配置模型。请在 Settings 中添加模型。".to_string(),
    };

    // Call LLM
    call_agent_llm(&api_url, &api_key, &model, &system_prompt, task, &tools_json, org, agents, models, global_config, 0)
}

fn call_agent_llm(
    api_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    task: &str,
    tools_json: &Value,
    org: &crate::config::OrgChart,
    agents: &std::collections::HashMap<String, crate::config::AgentConfig>,
    models: &[crate::config::ModelEntry],
    global_config: &crate::config::Config,
    depth: usize,
) -> String {
    if depth > 5 {
        return "委派层级过深，已终止。".to_string();
    }

    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));
    let messages = json!([
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": task}
    ]);

    let body = json!({
        "model": model,
        "messages": messages,
        "tools": tools_json,
        "tool_choice": "auto"
    });

    let result = match ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(resp) => resp.into_string().unwrap_or_default(),
        Err(ureq::Error::Status(_, resp)) => {
            let err = resp.into_string().unwrap_or_default();
            return format!("LLM调用失败: {}", err);
        }
        Err(e) => return format!("LLM调用失败: {}", e),
    };

    let resp_json: Value = match serde_json::from_str(&result) {
        Ok(v) => v,
        Err(e) => return format!("解析响应失败: {}", e),
    };

    let choice = &resp_json["choices"][0];
    let msg = &choice["message"];

    // Handle tool calls (including recursive delegation)
    if let Some(tc) = msg.get("tool_calls") {
        if tc.as_array().map_or(false, |a| !a.is_empty()) {
            let mut tool_messages = vec![msg.clone()];

            if let Some(calls) = tc.as_array() {
                for call in calls {
                    let tool_name = call["function"]["name"].as_str().unwrap_or("");
                    let tool_args = call["function"]["arguments"].as_str().unwrap_or("{}");
                    let tool_call_id = call["id"].as_str().unwrap_or("");

                    let output = if tool_name == "delegate" {
                        // Recursive delegation
                        let args: Value = serde_json::from_str(tool_args).unwrap_or_default();
                        let target_agent = args["agent_name"].as_str().unwrap_or("");
                        let task_desc = args["task"].as_str().unwrap_or("");
                        tool_delegate(target_agent, task_desc, org, agents, models, global_config)
                    } else {
                        crate::tools::execute_tool(tool_name, tool_args, &[])
                    };

                    tool_messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": output
                    }));
                }
            }

            // Call LLM again with tool results
            let initial = json!([
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": task}
            ]);
            let mut new_messages = initial.as_array().unwrap().clone();

            for m in tool_messages {
                new_messages.push(m);
            }

            let body2 = json!({
                "model": model,
                "messages": new_messages,
                "tools": tools_json,
                "tool_choice": "auto"
            });

            let result2 = match ureq::post(&url)
                .set("Authorization", &format!("Bearer {}", api_key))
                .set("Content-Type", "application/json")
                .send_string(&body2.to_string())
            {
                Ok(resp) => resp.into_string().unwrap_or_default(),
                Err(e) => return format!("LLM调用失败: {}", e),
            };

            let resp2: Value = match serde_json::from_str(&result2) {
                Ok(v) => v,
                Err(e) => return format!("解析响应失败: {}", e),
            };

            return resp2["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("(empty)")
                .to_string();
        }
    }

    // No tool calls — direct reply
    msg["content"].as_str().unwrap_or("(empty)").to_string()
}

fn available_agents(org: &crate::config::OrgChart) -> String {
    org.positions
        .iter()
        .flat_map(|p| p.agents.clone())
        .collect::<Vec<_>>()
        .join("、")
}
