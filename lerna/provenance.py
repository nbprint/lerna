import copy
from dataclasses import dataclass, field
from typing import Any

from omegaconf import DictConfig, ListConfig, OmegaConf
from omegaconf.errors import OmegaConfBaseException

from lerna.core.default_element import ResultDefault
from lerna.core.override_parser.types import ListOperationType, Override
from lerna.plugins.config_source import ConfigResult, ConfigSource


@dataclass(frozen=True)
class CompositionSource:
    """Identify one config source used during composition."""

    identifier: str
    provider: str
    path: str
    config_path: str


@dataclass(frozen=True)
class SelectedDefault:
    """Describe one selected default in composition order."""

    config_path: str
    option: str | None
    package: str
    provider: str
    source: CompositionSource


@dataclass(frozen=True)
class NodeProvenance:
    """Describe the origin of one node or list item in the final config."""

    source_identifier: str
    source_config: str | None
    package: str
    source_key: str
    origin: str
    operation: int | None = None


@dataclass(frozen=True)
class RemovedItem:
    """Store a removed list value with its provenance."""

    value: Any
    provenance: NodeProvenance


@dataclass(frozen=True)
class CompositionOperation:
    """Describe one config-authored patch or command-line override."""

    name: str
    path: str
    origin: str
    source_config: str | None
    input: str | None
    previous: NodeProvenance | None
    removed_items: tuple[RemovedItem, ...] = ()


@dataclass
class CompositionResult:
    """Contain an unresolved config and metadata collected during composition."""

    config: DictConfig
    selected_defaults: tuple[SelectedDefault, ...]
    available_options: dict[str, tuple[str, ...]]
    node_provenance: dict[str, NodeProvenance]
    patch_operations: tuple[CompositionOperation, ...]
    cli_overrides: tuple[CompositionOperation, ...]
    sources: tuple[CompositionSource, ...]

    def resolved_copy(self) -> DictConfig:
        """Return an independently resolved copy of the composed config."""
        resolved = copy.deepcopy(self.config)
        OmegaConf.resolve(resolved)
        return resolved

    def provenance(self, path: str) -> NodeProvenance | None:
        """Return provenance for a dot path or indexed list path."""
        return self.node_provenance.get(path)


@dataclass
class _OverrideState:
    value: Any = None
    previous: NodeProvenance | None = None
    resolved_values: list[Any] = field(default_factory=list)
    item_provenance: list[NodeProvenance | None] = field(default_factory=list)
    candidate_values: list[Any] = field(default_factory=list)


class _CompositionTracker:
    def __init__(self) -> None:
        self.selected_defaults: list[SelectedDefault] = []
        self.available_options: dict[str, tuple[str, ...]] = {}
        self.node_provenance: dict[str, NodeProvenance] = {}
        self.patch_operations: list[CompositionOperation] = []
        self.cli_overrides: list[CompositionOperation] = []
        self.sources: dict[str, CompositionSource] = {}

    @staticmethod
    def _child_path(parent: str, key: str | int) -> str:
        if isinstance(key, int):
            return f"{parent}[{key}]"
        return f"{parent}.{key}" if parent else key

    @staticmethod
    def _source_identifier(result: ConfigResult, config_path: str) -> str:
        normalized = ConfigSource._normalize_file_name(config_path)
        if result.path.endswith("://"):
            return result.path + normalized
        return f"{result.path.rstrip('/')}/{normalized}"

    @staticmethod
    def _source_key(path: str, package: str) -> str:
        if not package:
            return path
        if path == package:
            return ""
        prefix = package + "."
        return path.removeprefix(prefix)

    def _remove_path(self, path: str) -> None:
        prefixes = (path + ".", path + "[")
        for existing in list(self.node_provenance):
            if existing == path or existing.startswith(prefixes):
                del self.node_provenance[existing]

    def _record_tree(self, node: Any, path: str, provenance: NodeProvenance, replace: bool) -> None:
        if replace and path:
            self._remove_path(path)
        if path:
            self.node_provenance[path] = provenance
        if isinstance(node, DictConfig):
            for key in node:
                child_path = self._child_path(path, key)
                child_provenance = NodeProvenance(
                    provenance.source_identifier,
                    provenance.source_config,
                    provenance.package,
                    self._child_path(provenance.source_key, key),
                    provenance.origin,
                    provenance.operation,
                )
                self._record_tree(node._get_node(key), child_path, child_provenance, replace=False)
        elif isinstance(node, ListConfig):
            for index in range(len(node)):
                child_path = self._child_path(path, index)
                child_provenance = NodeProvenance(
                    provenance.source_identifier,
                    provenance.source_config,
                    provenance.package,
                    self._child_path(provenance.source_key, index),
                    provenance.origin,
                    provenance.operation,
                )
                self._record_tree(node._get_node(index), child_path, child_provenance, replace=False)

    def record_default(self, default: ResultDefault, result: ConfigResult) -> None:
        assert default.config_path is not None
        package = default.package or ""
        identifier = self._source_identifier(result, default.config_path)
        source = CompositionSource(identifier, result.provider, result.path, default.config_path)
        self.sources[identifier] = source
        option = default.config_path.rsplit("/", 1)[-1] if default.override_key is not None else None
        self.selected_defaults.append(SelectedDefault(default.config_path, option, package, result.provider, source))

        def record(node: Any, path: str) -> None:
            source_key = self._source_key(path, package)
            provenance = NodeProvenance(identifier, default.config_path, package, source_key, "config")
            replace = not isinstance(node, DictConfig)
            self._record_tree(node, path, provenance, replace=replace)

        if isinstance(result.config, DictConfig):
            for key in result.config:
                record(result.config._get_node(key), str(key))

    def set_available_options(self, group: str, options: list[str]) -> None:
        self.available_options[group] = tuple(options)

    @staticmethod
    def _operation_name(override: Override) -> str:
        if override.is_list_extend():
            input_line = override.input_line or ""
            expression = input_line.split("=", 1)[-1]
            authored_name = expression.split("(", 1)[0]
            known_names = {
                "append",
                "prepend",
                "insert",
                "pop",
                "remove_at",
                "remove",
                "remove_value",
                "clear",
                "list_clear",
                "append_unique",
                "remove_all",
                "extend",
                "extend_from",
                "delete_slice",
            }
            if authored_name in known_names:
                return authored_name
            names = {
                None: "append",
                ListOperationType.APPEND: "append",
                ListOperationType.PREPEND: "prepend",
                ListOperationType.INSERT: "insert",
                ListOperationType.REMOVE_AT: "pop",
                ListOperationType.REMOVE_VALUE: "remove",
                ListOperationType.CLEAR: "clear",
                ListOperationType.APPEND_UNIQUE: "append_unique",
                ListOperationType.REMOVE_ALL: "remove_all",
                ListOperationType.EXTEND_FROM: "extend",
                ListOperationType.DELETE_SLICE: "delete_slice",
            }
            return names[override.list_operation]
        if override.is_delete():
            return "delete"
        if override.is_force_add():
            return "force_add"
        if override.is_add():
            return "add"
        return "change"

    @staticmethod
    def _resolved_candidate(cfg: DictConfig, key: str, value: Any) -> Any:
        candidate_cfg = copy.deepcopy(cfg)
        target = OmegaConf.select(candidate_cfg, key, throw_on_missing=False)
        if isinstance(target, ListConfig):
            target.append(value)
            candidate = target[-1]
            return OmegaConf.to_container(candidate, resolve=True) if OmegaConf.is_config(candidate) else candidate
        return value

    def before_override(self, override: Override, cfg: DictConfig) -> _OverrideState:
        key = override.key_or_group
        state = _OverrideState(previous=self.node_provenance.get(key))
        try:
            value = OmegaConf.select(cfg, key, throw_on_missing=False)
        except OmegaConfBaseException:
            return state
        state.value = copy.deepcopy(OmegaConf.to_container(value, resolve=False) if OmegaConf.is_config(value) else value)
        if isinstance(value, ListConfig):
            state.resolved_values = list(OmegaConf.to_container(value, resolve=True))
            state.item_provenance = [self.node_provenance.get(self._child_path(key, index)) for index in range(len(value))]
            override_value = override.value()
            candidates = override_value if isinstance(override_value, list) else []
            state.candidate_values = [self._resolved_candidate(cfg, key, candidate) for candidate in candidates]
        return state

    def _operation_provenance(self, override: Override, origin: str, index: int) -> NodeProvenance:
        source_config = override.source_config_path if origin == "patch" else None
        source = next((source for source in self.sources.values() if source.config_path == source_config), None)
        identifier = source.identifier if source is not None else "command-line"
        package = getattr(override, "source_package", None) or ""
        return NodeProvenance(identifier, source_config, package, override.key_or_group, origin, index)

    def after_override(self, override: Override, cfg: DictConfig, origin: str, state: _OverrideState) -> None:
        records = self.patch_operations if origin == "patch" else self.cli_overrides
        operation_index = len(records)
        provenance = self._operation_provenance(override, origin, operation_index)
        key = override.key_or_group
        removed: list[RemovedItem] = []

        if override.is_delete():
            self._remove_path(key)
        else:
            current = OmegaConf.select(cfg, key, throw_on_missing=False)
            if override.is_list_extend() and isinstance(current, ListConfig):
                origins = list(state.item_provenance)
                operation = override.list_operation
                if operation in (None, ListOperationType.APPEND, ListOperationType.APPEND_UNIQUE, ListOperationType.EXTEND_FROM):
                    origins.extend([provenance] * (len(current) - len(origins)))
                elif operation == ListOperationType.PREPEND:
                    origins = [provenance] * (len(current) - len(origins)) + origins
                elif operation == ListOperationType.INSERT:
                    index = override.list_index or 0
                    if index < 0:
                        index = len(state.item_provenance) + index + 1
                    origins[index:index] = [provenance] * (len(current) - len(origins))
                elif operation == ListOperationType.REMOVE_AT:
                    index = override.list_index or 0
                    if index < 0:
                        index = len(origins) + index
                    if 0 <= index < len(origins):
                        removed_origin = origins.pop(index) or provenance
                        removed.append(RemovedItem(state.resolved_values[index], removed_origin))
                elif operation == ListOperationType.DELETE_SLICE:
                    start = override.list_index if override.list_index is not None else 0
                    normalized_start, normalized_stop, _ = slice(start, override.list_end_index).indices(len(origins))
                    removed = [
                        RemovedItem(state.resolved_values[index], origins[index] or provenance) for index in range(normalized_start, normalized_stop)
                    ]
                    del origins[normalized_start:normalized_stop]
                elif operation == ListOperationType.REMOVE_VALUE:
                    values = list(state.resolved_values)
                    for candidate in state.candidate_values:
                        if candidate in values:
                            index = values.index(candidate)
                            removed.append(RemovedItem(values.pop(index), origins.pop(index) or provenance))
                elif operation == ListOperationType.REMOVE_ALL:
                    values = list(state.resolved_values)
                    for index in range(len(values) - 1, -1, -1):
                        if values[index] in state.candidate_values:
                            removed.append(RemovedItem(values[index], origins[index] or provenance))
                            del values[index]
                            del origins[index]
                    removed.reverse()
                elif operation == ListOperationType.CLEAR:
                    removed = [RemovedItem(value, item_origin or provenance) for value, item_origin in zip(state.resolved_values, origins)]
                    origins = []

                self._remove_path(key)
                self.node_provenance[key] = provenance
                for index in range(len(current)):
                    item_provenance = origins[index] or provenance
                    self._record_tree(current._get_node(index), self._child_path(key, index), item_provenance, replace=False)
            else:
                self._record_tree(current, key, provenance, replace=True)

        records.append(
            CompositionOperation(
                self._operation_name(override),
                key,
                origin,
                override.source_config_path if origin == "patch" else None,
                override.input_line,
                state.previous,
                tuple(removed),
            )
        )

    def result(self, config: DictConfig) -> CompositionResult:
        return CompositionResult(
            config=config,
            selected_defaults=tuple(self.selected_defaults),
            available_options=dict(self.available_options),
            node_provenance=dict(self.node_provenance),
            patch_operations=tuple(self.patch_operations),
            cli_overrides=tuple(self.cli_overrides),
            sources=tuple(self.sources.values()),
        )
