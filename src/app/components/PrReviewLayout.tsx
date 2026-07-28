import { Box } from "ink";
import type {
  AdoRepository,
  NormalizedPullRequest,
  PullRequestFilters,
} from "../../api/types.js";
import type { CommentActivity } from "../hooks/usePrCommentActivity.js";
import type { PullRequestDetailState } from "../hooks/usePullRequestDetail.js";
import type { TerminalSize } from "../hooks/useTerminalSize.js";
import { computeLayoutGeometry } from "../utils/layoutGeometry.js";
import type { NormalizedSelection } from "../utils/textSelection.js";
import { FilterModal, filtersToModalState, type FilterModalState } from "./FilterModal.js";
import { Header } from "./Header.js";
import { PrListPanel } from "./PrListPanel.js";
import { PrRightPanel, type CommentStatusFilter, type RightPanelMode } from "./PrRightPanel.js";

export type PanelFocus = "list" | "detail";

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
  leftWidthRatio: number;
  commentActivity: Map<string, CommentActivity>;
  rightSelection: NormalizedSelection | null;
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
  leftWidthRatio,
  commentActivity,
  rightSelection,
}: PrReviewLayoutProps) {
  const { panelHeight: mainHeight, leftWidth, rightWidth } = computeLayoutGeometry(terminalSize, {
    hasError: Boolean(error),
    leftWidthRatio,
  });

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
      {filterModalOpen ? (
        <FilterModal
          repositories={repositories}
          initial={filtersToModalState(filters)}
          onApply={onApplyFilters}
          onClose={onCloseFilterModal}
        />
      ) : (
        <Box flexDirection="row">
          <PrListPanel
            pullRequests={pullRequests}
            selectedIndex={selectedIndex}
            filterLabels={filterLabels}
            width={leftWidth}
            height={mainHeight}
            focused={focus === "list"}
            commentActivity={commentActivity}
            searchMode={searchMode}
            searchQuery={searchQuery}
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
            selection={rightSelection}
          />
        </Box>
      )}
    </Box>
  );
}
