import { Box, Text, useApp, useInput } from "ink";
import open from "open";
import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  AppScreen,
  NormalizedPullRequest,
  Project,
  ProviderId,
  PullRequestFilters,
  Repository,
} from "../api/types.js";
import { isConfiguredProvider } from "../api/types.js";
import {
  getLastProject,
  getProviderDisplayContext,
  loadConfig,
  saveProviderConfig,
  updateLastProject,
} from "../config/store.js";
import { getProvider } from "../providers/registry.js";
import type { ProviderClient, ProviderDefinition } from "../providers/types.js";
import { DetailPanel } from "./components/DetailPanel.js";
import {
  FilterModal,
  filtersToLabels,
  filtersToModalState,
  modalStateToFilters,
  type FilterModalState,
} from "./components/FilterModal.js";
import { Header } from "./components/Header.js";
import { AdoProviderSetup, ProviderPicker } from "./components/ProviderPicker.js";
import { FilterBar, PrTable } from "./components/PrTable.js";
import { ProjectPicker } from "./components/ProjectPicker.js";
import { StatusBar } from "./components/StatusBar.js";

interface AppProps {
  initialProject?: string;
  initialFilters: PullRequestFilters;
}

function resolveInitialScreen(
  configured: boolean,
  initialProject?: string,
): AppScreen {
  if (!configured) return "providerSelect";
  if (initialProject) return "prList";
  return "projectSelect";
}

export function App({ initialProject, initialFilters }: AppProps) {
  const { exit } = useApp();
  const [client, setClient] = useState<ProviderClient | null>(null);
  const [currentUserId, setCurrentUserId] = useState<string | undefined>();
  const [screen, setScreen] = useState<AppScreen>("providerSelect");
  const [setupProvider, setSetupProvider] = useState<ProviderDefinition | null>(null);
  const [setupLoading, setSetupLoading] = useState(false);
  const [setupDetecting, setSetupDetecting] = useState(false);
  const [setupError, setSetupError] = useState<string>();

  const [lastProject, setLastProject] = useState<string | undefined>();
  const [selectedProject, setSelectedProject] = useState<string | undefined>(initialProject);
  const [displayContext, setDisplayContext] = useState("");

  const [projects, setProjects] = useState<Project[]>([]);
  const [projectsLoading, setProjectsLoading] = useState(false);
  const [projectsError, setProjectsError] = useState<string>();
  const [projectSearch, setProjectSearch] = useState("");

  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [pullRequests, setPullRequests] = useState<NormalizedPullRequest[]>([]);
  const [prsLoading, setPrsLoading] = useState(false);
  const [prsError, setPrsError] = useState<string>();
  const [refreshedAt, setRefreshedAt] = useState<Date>();
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filters, setFilters] = useState<PullRequestFilters>(initialFilters);
  const [searchMode, setSearchMode] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterModalOpen, setFilterModalOpen] = useState(false);
  const [bootstrapped, setBootstrapped] = useState(false);

  const initClient = useCallback(
    async (providerId: ProviderId) => {
      const provider = getProvider(providerId);
      if (!provider) throw new Error(`Unknown provider: ${providerId}`);

      const config = await loadConfig();
      const providerConfig = config.providers[providerId] ?? {};
      const resolvedToken = await provider.resolveToken();
      const newClient = provider.createClient(resolvedToken, providerConfig);
      const user = await newClient.getCurrentUser();

      setClient(newClient);
      setCurrentUserId(user.id);
      setDisplayContext(getProviderDisplayContext(config));
      setLastProject(getLastProject(config));
      return newClient;
    },
    [],
  );

  useEffect(() => {
    if (bootstrapped) return;
    void (async () => {
      const config = await loadConfig();
      const configured = isConfiguredProvider(config);
      setScreen(resolveInitialScreen(configured, initialProject));

      if (configured && config.activeProvider) {
        try {
          await initClient(config.activeProvider);
        } catch (error) {
          setProjectsError(error instanceof Error ? error.message : String(error));
          setScreen("providerSelect");
        }
      }
      setBootstrapped(true);
    })();
  }, [bootstrapped, initClient, initialProject]);

  const loadProjects = useCallback(async () => {
    if (!client) return;
    setProjectsLoading(true);
    setProjectsError(undefined);
    try {
      const config = await loadConfig();
      setLastProject(getLastProject(config));
      setDisplayContext(getProviderDisplayContext(config));
      const list = await client.listProjects();
      setProjects(list);
    } catch (error) {
      setProjectsError(error instanceof Error ? error.message : String(error));
    } finally {
      setProjectsLoading(false);
    }
  }, [client]);

  const loadPullRequests = useCallback(
    async (project: string, nextFilters: PullRequestFilters) => {
      if (!client) return;
      setPrsLoading(true);
      setPrsError(undefined);
      try {
        const [repos, prs] = await Promise.all([
          client.listRepositories(project),
          client.listPullRequests(project, nextFilters, currentUserId),
        ]);
        setRepositories(repos);
        setPullRequests(prs);
        setRefreshedAt(new Date());
        setSelectedIndex(0);
      } catch (error) {
        setPrsError(error instanceof Error ? error.message : String(error));
      } finally {
        setPrsLoading(false);
      }
    },
    [client, currentUserId],
  );

  useEffect(() => {
    if (screen === "projectSelect" && client) {
      void loadProjects();
    }
  }, [screen, client, loadProjects]);

  useEffect(() => {
    if (selectedProject && screen === "prList" && client) {
      void loadPullRequests(selectedProject, filters);
    }
  }, [selectedProject, screen, filters, loadPullRequests, client]);

  const displayedPrs = useMemo(() => {
    if (!filters.search && !searchQuery) return pullRequests;
    const q = (filters.search ?? searchQuery).toLowerCase();
    return pullRequests.filter((pr) => {
      const haystack = `${pr.repoName} ${pr.title} ${pr.author}`.toLowerCase();
      return haystack.includes(q);
    });
  }, [pullRequests, filters.search, searchQuery]);

  const safeSelectedIndex =
    displayedPrs.length === 0 ? 0 : Math.min(selectedIndex, displayedPrs.length - 1);
  const selectedPr = displayedPrs[safeSelectedIndex];
  const filterLabels = filtersToLabels({
    ...filters,
    search: filters.search ?? (searchQuery || undefined),
  });

  const handleProviderSelect = (provider: ProviderDefinition) => {
    if (!provider.enabled) return;

    if (provider.id === "azure-devops") {
      setSetupProvider(provider);
      setSetupError(undefined);
      setSetupDetecting(true);
      setScreen("providerSetup");
    }
  };

  const handleAdoSubmit = async (org: string) => {
    if (!org) {
      setSetupError("Organization name cannot be empty.");
      return;
    }

    const provider = getProvider("azure-devops");
    if (!provider) return;

    setSetupLoading(true);
    setSetupError(undefined);
    try {
      const resolvedToken = await provider.resolveToken();
      const testClient = provider.createClient(resolvedToken, { organization: org });
      await testClient.getCurrentUser();
      await testClient.listProjects();

      await saveProviderConfig("azure-devops", { organization: org });
      setClient(testClient);
      setDisplayContext(org);
      const user = await testClient.getCurrentUser();
      setCurrentUserId(user.id);
      setScreen(initialProject ? "prList" : "projectSelect");
      setSetupProvider(null);
    } catch (error) {
      setSetupError(
        error instanceof Error
          ? error.message
          : "Could not connect. Check the org name and authentication.",
      );
    } finally {
      setSetupLoading(false);
    }
  };

  const handleProjectSelect = async (project: Project) => {
    setSelectedProject(project.name);
    await updateLastProject(project.name);
    setLastProject(project.name);
    setScreen("prList");
    setSearchQuery("");
    setSearchMode(false);
  };

  const handleApplyFilters = (state: FilterModalState) => {
    if (!currentUserId) return;
    const next = modalStateToFilters(state, currentUserId);
    if (searchQuery) next.search = searchQuery;
    setFilters(next);
    setFilterModalOpen(false);
  };

  useInput((input, key) => {
    if (screen === "providerSelect" || screen === "providerSetup" || screen === "projectSelect") {
      return;
    }
    if (screen !== "prList" || filterModalOpen) return;

    if (searchMode) {
      if (key.return) {
        setSearchMode(false);
        setFilters((f) => ({ ...f, search: searchQuery || undefined }));
        return;
      }
      if (key.escape) {
        setSearchMode(false);
        setSearchQuery("");
        setFilters((f) => ({ ...f, search: undefined }));
        return;
      }
      if (key.backspace || key.delete) {
        setSearchQuery((q) => q.slice(0, -1));
        return;
      }
      if (input.length === 1 && !key.ctrl && !key.meta) {
        setSearchQuery((q) => q + input);
      }
      return;
    }

    if (key.upArrow || input === "k") {
      setSelectedIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow || input === "j") {
      setSelectedIndex((i) => Math.min(Math.max(displayedPrs.length - 1, 0), i + 1));
      return;
    }
    if ((key.return || input === "o") && selectedPr) {
      void open(selectedPr.url);
      return;
    }
    if (input === "p") {
      setScreen("projectSelect");
      setProjectSearch("");
      void loadProjects();
      return;
    }
    if (input === "f") {
      setFilterModalOpen(true);
      return;
    }
    if (input === "/") {
      setSearchMode(true);
      return;
    }
    if (input === "r" && selectedProject) {
      void loadPullRequests(selectedProject, filters);
      return;
    }
    if (input === "q" || (key.ctrl && input === "c")) {
      exit();
    }
  });

  if (!bootstrapped) {
    return (
      <Box>
        <Text dimColor>Loading…</Text>
      </Box>
    );
  }

  if (screen === "providerSelect") {
    return (
      <Box flexDirection="column">
        <ProviderPicker
          onSelect={handleProviderSelect}
          onQuit={exit}
        />
        <StatusBar screen="providerSelect" />
      </Box>
    );
  }

  if (screen === "providerSetup" && setupProvider?.id === "azure-devops") {
    return (
      <Box flexDirection="column">
        <AdoProviderSetup
          loading={setupLoading}
          error={setupError}
          onSubmit={(org) => void handleAdoSubmit(org)}
          onQuit={exit}
          onDetectingChange={setSetupDetecting}
        />
        <StatusBar screen="providerSetup" setupDetecting={setupDetecting} />
      </Box>
    );
  }

  if (screen === "projectSelect") {
    return (
      <Box flexDirection="column">
        <ProjectPicker
          organization={displayContext}
          projects={projects}
          lastProject={lastProject}
          loading={projectsLoading}
          error={projectsError}
          search={projectSearch}
          onSearchChange={setProjectSearch}
          onSelect={(p) => void handleProjectSelect(p)}
          onRefresh={() => void loadProjects()}
          onQuit={exit}
        />
        <StatusBar screen="projectSelect" />
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      <Header
        organization={displayContext}
        project={selectedProject ?? ""}
        count={displayedPrs.length}
        loading={prsLoading}
        error={prsError}
        refreshedAt={refreshedAt}
      />
      <FilterBar filters={filterLabels} />
      {searchMode && (
        <Box marginTop={1}>
          <TextSearchPrompt query={searchQuery} />
        </Box>
      )}
      {filterModalOpen ? (
        <FilterModal
          repositories={repositories}
          initial={filtersToModalState(filters)}
          onApply={handleApplyFilters}
          onClose={() => setFilterModalOpen(false)}
        />
      ) : (
        <PrTable pullRequests={displayedPrs} selectedIndex={safeSelectedIndex} />
      )}
      <DetailPanel pullRequest={selectedPr} />
      <StatusBar screen="prList" modalOpen={filterModalOpen} />
    </Box>
  );
}

function TextSearchPrompt({ query }: { query: string }) {
  return (
    <Box>
      <Text dimColor>/ search: </Text>
      <Text>{query}</Text>
      <Text dimColor> (Enter apply · Esc clear)</Text>
    </Box>
  );
}
