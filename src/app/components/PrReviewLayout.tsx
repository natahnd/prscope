import { Box, Text } from "ink";
import type {
  AdoRepository,
  NormalizedPullRequest,
  PullRequestFilters,
} from "../../api/types.js";
import type { PullRequestDetailState } from "../hooks/usePullRequestDetail.js";
import type { TerminalSize } from "../hooks/useTerminalSize.js";
import { FilterModal, filtersToModalState, type FilterModalState } from "./FilterModal.js";
import { Header } from "./Header.js";
import { PrListPanel } from "./PrListPanel.js";
import { PrRightPanel, type CommentStatusFilter, type RightPanelMode } from "./PrRightPanel.js";

export type PanelFocus = "list" | "detail";

const STATUS_BAR_HEIGHT = 3;
const HEADER_HEIGHT = 3;
const ROW_GAP = 1;
const SEARCH_LINE_HEIGHT = 2;
const ERROR_LINE_HEIGHT = 2;
const LEFT_WIDTH_RATIO = 0.35;
const MIN_PANEL_WIDTH = 20;
const MIN_PANEL_HEIGHT = 5;

interface PrReviewLayoutProps {
  terminalSize: TerminalSize;
  organization: string;
  project: string;
  pullRequests: NormalizedPullRequest[];
  selectedIndex: number;
  loading: boolean;
  error?: string;
  refreshedAt?: Date;
  filterLabels: string[];
  searchMode: boolean;
  searchQuery: string;
  filterModalOpen: boolean;
  repositories: AdoRepository[];
  filters: PullRequestFilters;
  onApplyFilters: (state: FilterModalState) => void;
  onCloseFilterModal: () => void;
  focus: PanelFocus;
  detailState: PullRequestDetailState;
  rightMode: RightPanelMode;
  commentStatusFilter: CommentStatusFilter;
  rightScroll: number;
  onRightMaxScrollChange: (maxScroll: number) => void;
}

export function PrReviewLayout({
  terminalSize,
  organization,
  project,
  pullRequests,
  selectedIndex,
  loading,
  error,
  refreshedAt,
  filterLabels,
  searchMode,
  searchQuery,
  filterModalOpen,
  repositories,
  filters,
  onApplyFilters,
  onCloseFilterModal,
  focus,
  detailState,
  rightMode,
  commentStatusFilter,
  rightScroll,
  onRightMaxScrollChange,
}: PrReviewLayoutProps) {
  const reservedTop =
    HEADER_HEIGHT +
    (error ? ERROR_LINE_HEIGHT : 0) +
    (searchMode ? SEARCH_LINE_HEIGHT : 0) +
    ROW_GAP;
  const mainHeight = Math.max(MIN_PANEL_HEIGHT, terminalSize.rows - reservedTop - STATUS_BAR_HEIGHT);

  const leftWidth = Math.max(MIN_PANEL_WIDTH, Math.floor(terminalSize.columns * LEFT_WIDTH_RATIO));
  const rightWidth = Math.max(MIN_PANEL_WIDTH, terminalSize.columns - leftWidth);

  const selectedPr = pullRequests[selectedIndex];

  return (
    <Box flexDirection="column">
      <Header
        organization={organization}
        project={project}
        count={pullRequests.length}
        loading={loading}
        error={error}
        refreshedAt={refreshedAt}
      />
      {searchMode && (
        <Box marginTop={1}>
          <TextSearchPrompt query={searchQuery} />
        </Box>
      )}
      {filterModalOpen ? (
        <FilterModal
          repositories={repositories}
          initial={filtersToModalState(filters)}
          onApply={onApplyFilters}
          onClose={onCloseFilterModal}
        />
      ) : (
        <Box flexDirection="row" marginTop={1}>
          <PrListPanel
            pullRequests={pullRequests}
            selectedIndex={selectedIndex}
            filterLabels={filterLabels}
            width={leftWidth}
            height={mainHeight}
            focused={focus === "list"}
          />
          <PrRightPanel
            pullRequest={selectedPr}
            detailState={detailState}
            mode={rightMode}
            commentStatusFilter={commentStatusFilter}
            scrollOffset={rightScroll}
            width={rightWidth}
            height={mainHeight}
            focused={focus === "detail"}
            onMaxScrollChange={onRightMaxScrollChange}
          />
        </Box>
      )}
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
