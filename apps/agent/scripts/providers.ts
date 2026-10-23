/** Generated from the pinned Pi SDK, without resolving auth or making requests. */
import { writeFileSync } from "node:fs";
import { builtinProviders } from "@earendil-works/pi-ai/providers/all";
import { createModels } from "@earendil-works/pi-ai";
const providers = builtinProviders().map((p) => {
  const models = createModels(); models.setProvider(p);
  return { id: p.id, name: p.name, chat: models.getModels(p.id).length > 0,
    methods: [...(p.auth?.apiKey?.login ? [{ id: "api_key", label: p.auth.apiKey.name }] : []), ...(p.auth?.oauth ? [{ id: "oauth", label: p.auth.oauth.loginLabel ?? p.auth.oauth.name }] : [])] };
});
providers.push({ id: "newapi", name: "OpenAI-compatible", chat: true, methods: [{ id: "api_key", label: "API Key" }] });
writeFileSync(new URL("../providers.json", import.meta.url), JSON.stringify(providers, null, 2) + "\n");
