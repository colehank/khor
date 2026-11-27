import type { NetworkPolicy, NetworkSettings, RelayChoices, RelayWant } from "../../src/api";
import { FixtureRefusal } from "./navigation";
import { labNetwork } from "./world";

/** The browser preview stores example settings in memory. It never dials a relay. */
export function networkSettingsFixture() {
  const settings = structuredClone(labNetwork);
  let keyed = new Set<string>();

  function relays(): RelayChoices {
    const { policy, relays: entries, preferred } = settings.profile;
    const mine = entries.map(relay => relay.url);
    const in_use = [
      ...(policy.khor_relays ? ["https://shared-relay.example.org"] : []),
      ...(policy.n0_relays ? ["https://fallback-relay.example.org"] : []),
      ...mine,
    ];
    return {
      mine, in_use, preferred,
      keyed: mine.filter(url => keyed.has(url)),
      only_mine: !policy.khor_relays && !policy.n0_relays,
    };
  }

  function saved() {
    settings.pending_restart = true;
    const { preferred, in_use } = relays();
    if (preferred && !in_use.includes(preferred)) settings.profile.preferred = null;
    return settings;
  }

  return {
    settings: () => settings,
    relays,
    setPolicy(policy: NetworkPolicy) {
      settings.explicit = true;
      settings.profile.policy = policy;
      return saved();
    },
    setRelays(wants: RelayWant[]) {
      const previous = settings.profile.relays;
      settings.profile.relays = wants.map(({ url, qad }) => ({
        url, qad: qad ?? previous.find(relay => relay.url === url)?.qad ?? null,
      }));
      keyed = new Set(wants.filter(({url, token}) =>
        token !== undefined && token !== null ? token.trim() : keyed.has(url),
      ).map(({url}) => url));
      saved();
      return relays();
    },
    setPreferred(url: string | null) {
      settings.profile.preferred = url;
      saved();
      return null;
    },
    importProfile(input: string) {
      let profile: NetworkSettings["profile"];
      try {
        profile = JSON.parse(input.trim()) as NetworkSettings["profile"];
        if (profile.version !== 1 || !Array.isArray(profile.relays)
          || !Array.isArray(profile.policy.pkarr) || !Array.isArray(profile.policy.dns_origins)
          || profile.relays.some(relay => typeof relay.url !== "string")) throw new Error();
      } catch {
        throw new FixtureRefusal("Lab 预览可导入从「查看 JSON」复制的示例配置。");
      }
      settings.explicit = true;
      settings.profile = profile;
      return saved();
    },
  };
}
