// WebMCP tools for the airgap website. Kept as a small standalone script, outside the app bundle, so agents
// and scanners find the registration without parsing the whole app. Tool data comes from /agent/knowledge.json,
// which the build generates from src/agent/knowledge.ts.
(function () {
  "use strict";
  if (!("modelContext" in navigator) || !navigator.modelContext) return;

  var reply = function (value) {
    return { content: [{ type: "text", text: typeof value === "string" ? value : JSON.stringify(value, null, 2) }] };
  };
  var fail = function (message) {
    var r = reply(message);
    r.isError = true;
    return r;
  };

  function tools(k) {
    var platforms = k.install.map(function (i) { return i.platform; });
    var clients = k.clientSetup.map(function (c) { return c.client; });
    var pages = k.pages.map(function (p) { return p.path; });
    return [
      {
        name: "airgap_search_docs",
        description: "Search the airgap website and docs. Returns the best matching sections with their URLs.",
        inputSchema: {
          type: "object",
          required: ["query"],
          properties: {
            query: { type: "string", description: "What to look for, e.g. 'verify a pull request'." },
            limit: { type: "integer", minimum: 1, maximum: 10 },
          },
        },
        annotations: { readOnlyHint: true },
        execute: function (args) {
          var query = String((args && args.query) || "").trim();
          if (!query) return fail("query is required");
          var limit = Math.min(Math.max(Number(args.limit) || 5, 1), 10);
          var url = "/api/v1/search?q=" + encodeURIComponent(query) + "&limit=" + limit;
          return fetch(url, { headers: { accept: "application/json" } })
            .then(function (res) {
              if (!res.ok) throw new Error(String(res.status));
              return res.json().then(reply);
            })
            .catch(function () {
              var words = query.toLowerCase().split(/\W+/).filter(Boolean);
              var hits = k.tools.filter(function (t) {
                var text = (t.name + " " + t.summary + " " + t.useWhen).toLowerCase();
                return words.some(function (w) { return text.indexOf(w) !== -1; });
              });
              return reply({
                query: query,
                results: hits.slice(0, limit).map(function (t) {
                  return { name: t.name, summary: t.summary, url: k.site.origin + "/docs#tools" };
                }),
              });
            });
        },
      },
      {
        name: "airgap_get_install_command",
        description: "Get the command that installs the airgap binary on a given platform.",
        inputSchema: { type: "object", required: ["platform"], properties: { platform: { type: "string", enum: platforms } } },
        annotations: { readOnlyHint: true },
        execute: function (args) {
          var hit = k.install.find(function (i) { return i.platform === (args && args.platform); });
          if (!hit) return fail("Unknown platform. Use one of: " + platforms.join(", "));
          var next = hit.platform === "npm" ? "Then run: npx -y airgap-mcp setup --yes && npx -y airgap-mcp doctor" : "Then run: airgap setup --yes && airgap doctor";
          return reply(Object.assign({}, hit, { next: next }));
        },
      },
      {
        name: "airgap_get_client_setup",
        description: "Get the setup command or MCP JSON config that registers airgap with an MCP client.",
        inputSchema: { type: "object", required: ["client"], properties: { client: { type: "string", enum: clients } } },
        annotations: { readOnlyHint: true },
        execute: function (args) {
          var hit = k.clientSetup.find(function (c) { return c.client === (args && args.client); });
          return hit ? reply(hit) : fail("Unknown client. Use one of: " + clients.join(", "));
        },
      },
      {
        name: "airgap_list_mcp_tools",
        description: "List the MCP tools the airgap server provides, with when to use each one and its input schema.",
        inputSchema: { type: "object", properties: {} },
        annotations: { readOnlyHint: true },
        execute: function () {
          return reply({ routing: k.routing, tools: k.tools });
        },
      },
      {
        name: "airgap_open_page",
        description: "Navigate this tab to a page of the airgap website, optionally scrolled to a section id.",
        inputSchema: {
          type: "object",
          required: ["page"],
          properties: {
            page: { type: "string", enum: pages },
            section: { type: "string", description: "Optional section id, e.g. 'tools' or 'faq'." },
          },
        },
        execute: function (args) {
          var page = k.pages.find(function (p) { return p.path === (args && args.page); });
          if (!page) return fail("Unknown page. Use one of: " + pages.join(", "));
          var hash = args.section ? "#" + encodeURIComponent(String(args.section)) : "";
          window.location.assign(page.path + hash);
          return reply("Opening " + page.title);
        },
      },
    ];
  }

  fetch("/agent/knowledge.json", { headers: { accept: "application/json" } })
    .then(function (res) {
      if (!res.ok) throw new Error("knowledge.json " + res.status);
      return res.json();
    })
    .then(function (k) {
      var list = tools(k);
      if (typeof navigator.modelContext.registerTool === "function") {
        list.forEach(function (tool) {
          navigator.modelContext.registerTool(tool);
        });
      } else if (typeof navigator.modelContext.provideContext === "function") {
        navigator.modelContext.provideContext({ tools: list });
      }
    })
    .catch(function (err) {
      console.warn("WebMCP registration failed", err);
    });
})();
