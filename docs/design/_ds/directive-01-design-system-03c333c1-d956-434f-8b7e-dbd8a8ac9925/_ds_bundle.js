/* @ds-bundle: {"format":4,"namespace":"Directive01DesignSystem_03c333","components":[{"name":"Button","sourcePath":"components/actions/Button.jsx"},{"name":"IconButton","sourcePath":"components/actions/IconButton.jsx"},{"name":"AuthorityBadge","sourcePath":"components/authority/AuthorityBadge.jsx"},{"name":"ClearanceGate","sourcePath":"components/authority/ClearanceGate.jsx"},{"name":"FocusRing","sourcePath":"components/authority/FocusRing.jsx"},{"name":"Handler","sourcePath":"components/authority/Handler.jsx"},{"name":"ApprovalChain","sourcePath":"components/command/ApprovalChain.jsx"},{"name":"CommandChain","sourcePath":"components/command/CommandChain.jsx"},{"name":"ConsequencePanel","sourcePath":"components/command/ConsequencePanel.jsx"},{"name":"Directive","sourcePath":"components/command/Directive.jsx"},{"name":"Avatar","sourcePath":"components/display/Avatar.jsx"},{"name":"Badge","sourcePath":"components/display/Badge.jsx"},{"name":"DataTable","sourcePath":"components/display/DataTable.jsx"},{"name":"KeyValue","sourcePath":"components/display/KeyValue.jsx"},{"name":"Panel","sourcePath":"components/display/Panel.jsx"},{"name":"Stat","sourcePath":"components/display/Stat.jsx"},{"name":"Dialog","sourcePath":"components/feedback/Dialog.jsx"},{"name":"EmptyState","sourcePath":"components/feedback/EmptyState.jsx"},{"name":"Notice","sourcePath":"components/feedback/Notice.jsx"},{"name":"Checkbox","sourcePath":"components/forms/Checkbox.jsx"},{"name":"Radio","sourcePath":"components/forms/Radio.jsx"},{"name":"Select","sourcePath":"components/forms/Select.jsx"},{"name":"Switch","sourcePath":"components/forms/Switch.jsx"},{"name":"TextField","sourcePath":"components/forms/TextField.jsx"},{"name":"AuditRecord","sourcePath":"components/records/AuditRecord.jsx"},{"name":"Contract","sourcePath":"components/records/Contract.jsx"},{"name":"Observation","sourcePath":"components/records/Observation.jsx"},{"name":"SubjectRecord","sourcePath":"components/records/SubjectRecord.jsx"}],"sourceHashes":{"components/actions/Button.jsx":"e8d79477deed","components/actions/IconButton.jsx":"7926f495bd64","components/authority/AuthorityBadge.jsx":"38a53ad0ece2","components/authority/ClearanceGate.jsx":"10e8bfe97743","components/authority/FocusRing.jsx":"651fa649892f","components/authority/Handler.jsx":"312eec2d28ef","components/command/ApprovalChain.jsx":"d9753fa151a4","components/command/CommandChain.jsx":"8f6f7d30a144","components/command/ConsequencePanel.jsx":"1685cfa0be33","components/command/Directive.jsx":"27af13f7ffd5","components/display/Avatar.jsx":"9e1414be7efe","components/display/Badge.jsx":"48f77c97a6f6","components/display/DataTable.jsx":"13b83a614608","components/display/KeyValue.jsx":"de63d92218f8","components/display/Panel.jsx":"975261e355bb","components/display/Stat.jsx":"334e84db7e43","components/feedback/Dialog.jsx":"7e8586a651f3","components/feedback/EmptyState.jsx":"749bce06d9b4","components/feedback/Notice.jsx":"531e62bac51a","components/forms/Checkbox.jsx":"4b59d6a17387","components/forms/Radio.jsx":"0d89152b53b5","components/forms/Select.jsx":"ef79d4c63c3d","components/forms/Switch.jsx":"bb1e8ea62b7e","components/forms/TextField.jsx":"b600bd88dacd","components/records/AuditRecord.jsx":"eac766a5fae6","components/records/Contract.jsx":"bdec039e7511","components/records/Observation.jsx":"2314b234bdad","components/records/SubjectRecord.jsx":"0a495334bd78","ui_kits/bureau/App.jsx":"7e41a9461556","ui_kits/bureau/Screens1.jsx":"088e4e1082af","ui_kits/bureau/Screens2.jsx":"9dcb3e1729e7","ui_kits/bureau/Screens3.jsx":"8e0781901bda"},"inlinedExternals":[],"unexposedExports":[{"name":"fieldInputStyle","sourcePath":"components/forms/TextField.jsx"},{"name":"fieldLabelStyle","sourcePath":"components/forms/TextField.jsx"}]} */

(() => {

const __ds_ns = (window.Directive01DesignSystem_03c333 = window.Directive01DesignSystem_03c333 || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/actions/Button.jsx
try { (() => {
const base = {
  fontFamily: 'var(--font-directive)',
  fontWeight: 600,
  letterSpacing: '0.07em',
  textTransform: 'uppercase',
  border: '1px solid transparent',
  borderRadius: 'var(--radius-standard)',
  cursor: 'pointer',
  transition: 'background var(--motion-standard), color var(--motion-standard), border-color var(--motion-standard)',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 8
};
const sizes = {
  sm: {
    fontSize: 12,
    padding: '6px 14px',
    minHeight: 28
  },
  md: {
    fontSize: 13,
    padding: '9px 20px',
    minHeight: 36
  }
};
const variants = {
  primary: {
    background: 'var(--surface-authority)',
    color: 'var(--text-inverse)',
    borderColor: 'var(--surface-authority)'
  },
  secondary: {
    background: 'transparent',
    color: 'var(--text-primary)',
    borderColor: 'var(--ink)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text-secondary)',
    borderColor: 'transparent'
  },
  control: {
    background: 'transparent',
    color: 'var(--directive-red)',
    borderColor: 'var(--directive-red)'
  }
};
const hovers = {
  primary: {
    background: 'var(--ink-soft)'
  },
  secondary: {
    background: 'var(--surface-inset)'
  },
  ghost: {
    background: 'var(--surface-inset)',
    color: 'var(--text-primary)'
  },
  control: {
    background: 'var(--directive-red)',
    color: 'var(--text-inverse)'
  }
};
function Button({
  variant = 'primary',
  size = 'md',
  disabled = false,
  onClick,
  children,
  style
}) {
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    disabled: disabled,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      ...base,
      ...sizes[size],
      ...variants[variant],
      ...(hover && !disabled ? hovers[variant] : null),
      ...(disabled ? {
        opacity: 0.45,
        cursor: 'not-allowed'
      } : null),
      ...style
    }
  }, children);
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/actions/Button.jsx", error: String((e && e.message) || e) }); }

// components/actions/IconButton.jsx
try { (() => {
function IconButton({
  label,
  disabled = false,
  onClick,
  children,
  style
}) {
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": label,
    title: label,
    disabled: disabled,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      width: 32,
      height: 32,
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: hover && !disabled ? 'var(--surface-inset)' : 'transparent',
      border: '1px solid var(--line)',
      borderRadius: 'var(--radius-standard)',
      color: 'var(--text-secondary)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.45 : 1,
      transition: 'background var(--motion-standard)',
      ...style
    }
  }, children);
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/actions/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/authority/AuthorityBadge.jsx
try { (() => {
const levels = {
  general: {
    color: 'var(--authority-general)',
    n: '01'
  },
  internal: {
    color: 'var(--authority-internal)',
    n: '02'
  },
  restricted: {
    color: 'var(--authority-restricted)',
    n: '03'
  },
  controlled: {
    color: 'var(--authority-controlled)',
    n: '04'
  },
  directive: {
    color: 'var(--authority-directive)',
    n: '05'
  }
};
function AuthorityBadge({
  level = 'general',
  role,
  scope,
  style
}) {
  const l = levels[level] || levels.general;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'stretch',
      border: '1px solid var(--line-strong)',
      borderRadius: 'var(--radius-standard)',
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.07em',
      textTransform: 'uppercase',
      overflow: 'hidden',
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      background: l.color,
      color: 'var(--text-inverse)',
      padding: '3px 7px',
      display: 'inline-flex',
      alignItems: 'center'
    }
  }, l.n), /*#__PURE__*/React.createElement("span", {
    style: {
      padding: '3px 8px',
      color: 'var(--text-secondary)',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      background: 'var(--surface-card)'
    }
  }, level, role && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-meta)'
    }
  }, "\xB7 ", role), scope && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-meta)'
    }
  }, "\xB7 ", scope)));
}
Object.assign(__ds_scope, { AuthorityBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/authority/AuthorityBadge.jsx", error: String((e && e.message) || e) }); }

// components/authority/ClearanceGate.jsx
try { (() => {
function ClearanceGate({
  level = 'CONTROLLED',
  title,
  owner,
  reason,
  onRequest,
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--line-strong)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      padding: '28px 32px',
      textAlign: 'center',
      maxWidth: 420,
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 12,
      fontWeight: 700,
      letterSpacing: '0.14em',
      color: 'var(--restricted)'
    }
  }, level), /*#__PURE__*/React.createElement("div", {
    "aria-hidden": "true",
    style: {
      width: 32,
      margin: '12px auto',
      borderTop: '1px solid var(--line-strong)'
    }
  }), title && /*#__PURE__*/React.createElement("h3", {
    style: {
      margin: '0 0 14px',
      fontSize: 16,
      fontWeight: 600
    }
  }, title), /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: 0,
      fontSize: 13,
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      textTransform: 'uppercase'
    }
  }, "Required"), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, level, " access")), owner && /*#__PURE__*/React.createElement("div", {
    style: {
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      textTransform: 'uppercase'
    }
  }, "Owner"), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, owner)), reason && /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      textTransform: 'uppercase'
    }
  }, "Reason"), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, reason))), onRequest && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onRequest,
    style: {
      marginTop: 18,
      fontFamily: 'var(--font-directive)',
      fontSize: 12,
      fontWeight: 600,
      letterSpacing: '0.07em',
      textTransform: 'uppercase',
      background: 'transparent',
      border: '1px solid var(--ink)',
      borderRadius: 'var(--radius-standard)',
      padding: '7px 16px',
      cursor: 'pointer',
      color: 'var(--text-primary)'
    }
  }, "Request access"));
}
Object.assign(__ds_scope, { ClearanceGate });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/authority/ClearanceGate.jsx", error: String((e && e.message) || e) }); }

// components/authority/FocusRing.jsx
try { (() => {
function FocusRing({
  state = 'selected',
  size = 96,
  rings = 2,
  children,
  style
}) {
  const color = {
    selected: 'var(--auburn)',
    watch: 'var(--observation-active)',
    escalated: 'var(--observation-escalated)',
    linked: 'var(--graphite)'
  }[state] || 'var(--auburn)';
  const c = size / 2;
  const inner = size * 0.28;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'relative',
      display: 'inline-flex',
      width: size,
      height: size,
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement("svg", {
    "aria-hidden": "true",
    width: size,
    height: size,
    viewBox: `0 0 ${size} ${size}`,
    style: {
      position: 'absolute',
      inset: 0
    }
  }, Array.from({
    length: Math.min(rings, 3)
  }).map((_, i) => /*#__PURE__*/React.createElement("circle", {
    key: i,
    cx: c,
    cy: c,
    r: inner + (c - inner - 1) / Math.min(rings, 3) * (i + 1),
    fill: "none",
    stroke: color,
    strokeOpacity: 0.55 - i * 0.18,
    strokeWidth: "1"
  }))), /*#__PURE__*/React.createElement("span", {
    style: {
      width: inner * 2,
      height: inner * 2,
      borderRadius: '50%',
      overflow: 'hidden',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, children));
}
Object.assign(__ds_scope, { FocusRing });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/authority/FocusRing.jsx", error: String((e && e.message) || e) }); }

// components/authority/Handler.jsx
try { (() => {
function Handler({
  agent,
  handler,
  boundary,
  directive,
  status = 'WORKING',
  restricted = [],
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--line)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      padding: '14px 16px',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      justifyContent: 'space-between',
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 14,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase'
    }
  }, agent), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: status === 'WORKING' || status === 'ACTIVE' ? 'var(--authorized)' : 'var(--text-meta)'
    }
  }, "\u25CF ", status)), /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: 0,
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: '10px 16px',
      fontSize: 13
    }
  }, [['HANDLER', handler], ['BOUNDARY', boundary], ['DIRECTIVE', directive], ['RESTRICTED', restricted.length ? restricted.join(', ') : '—']].map(([k, v]) => /*#__PURE__*/React.createElement("div", {
    key: k
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      marginBottom: 2
    }
  }, k), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, v)))));
}
Object.assign(__ds_scope, { Handler });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/authority/Handler.jsx", error: String((e && e.message) || e) }); }

// components/command/ApprovalChain.jsx
try { (() => {
const tone = {
  APPROVED: 'var(--authorized)',
  PENDING: 'var(--text-meta)',
  DECLINED: 'var(--restricted)',
  WAITING: 'var(--line-strong)'
};
function ApprovalChain({
  steps = [],
  style
}) {
  return /*#__PURE__*/React.createElement("ol", {
    style: {
      listStyle: 'none',
      margin: 0,
      padding: 0,
      display: 'flex',
      flexDirection: 'column',
      gap: 0,
      ...style
    }
  }, steps.map((s, i) => /*#__PURE__*/React.createElement("li", {
    key: i,
    style: {
      display: 'flex',
      gap: 14
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center'
    }
  }, /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 9,
      height: 9,
      borderRadius: '50%',
      marginTop: 5,
      flex: 'none',
      background: s.status === 'APPROVED' ? tone.APPROVED : 'transparent',
      border: `1px solid ${tone[s.status] || 'var(--line-strong)'}`
    }
  }), i < steps.length - 1 && /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 1,
      flex: 1,
      background: 'var(--line)'
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      paddingBottom: i < steps.length - 1 ? 18 : 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 12,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase'
    }
  }, s.name), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 12,
      color: tone[s.status] || 'var(--text-meta)',
      fontFamily: 'var(--font-directive)',
      letterSpacing: '0.06em'
    }
  }, s.status, s.time ? ` · ${s.time}` : '')))));
}
Object.assign(__ds_scope, { ApprovalChain });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/command/ApprovalChain.jsx", error: String((e && e.message) || e) }); }

// components/command/CommandChain.jsx
try { (() => {
function CommandChain({
  links = [],
  selected = -1,
  onSelect,
  style
}) {
  return /*#__PURE__*/React.createElement("ol", {
    style: {
      listStyle: 'none',
      margin: 0,
      padding: 0,
      ...style
    }
  }, links.map((l, i) => /*#__PURE__*/React.createElement("li", {
    key: i
  }, /*#__PURE__*/React.createElement("div", {
    role: onSelect ? 'button' : undefined,
    tabIndex: onSelect ? 0 : undefined,
    onClick: onSelect ? () => onSelect(l, i) : undefined,
    onKeyDown: onSelect ? e => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onSelect(l, i);
      }
    } : undefined,
    style: {
      display: 'inline-flex',
      flexDirection: 'column',
      padding: '8px 14px',
      border: i === selected ? '1px solid var(--ink)' : '1px solid var(--line)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      cursor: onSelect ? 'pointer' : 'default',
      minWidth: 180
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 13,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase'
    }
  }, l.name), l.detail && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 12,
      color: 'var(--text-meta)'
    }
  }, l.detail)), i < links.length - 1 && /*#__PURE__*/React.createElement("div", {
    "aria-hidden": "true",
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '2px 0 2px 24px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 1,
      height: 26,
      background: 'var(--line-strong)'
    }
  }), l.grant && /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 11,
      color: 'var(--text-meta)',
      fontStyle: 'normal'
    }
  }, "\u2193 ", l.grant)))));
}
Object.assign(__ds_scope, { CommandChain });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/command/CommandChain.jsx", error: String((e && e.message) || e) }); }

// components/command/ConsequencePanel.jsx
try { (() => {
function ConsequencePanel({
  consequences = [],
  irreversible = false,
  audit = true,
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--line-strong)',
      background: 'var(--surface-inset)',
      borderRadius: 'var(--radius-standard)',
      padding: '14px 18px',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: '0.1em',
      color: 'var(--text-meta)',
      marginBottom: 10
    }
  }, "CONSEQUENCES"), /*#__PURE__*/React.createElement("ul", {
    style: {
      margin: 0,
      padding: 0,
      listStyle: 'none',
      fontSize: 14,
      display: 'flex',
      flexDirection: 'column',
      gap: 6
    }
  }, consequences.map((c, i) => /*#__PURE__*/React.createElement("li", {
    key: i,
    style: {
      display: 'flex',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 10,
      borderTop: '1px solid var(--graphite)',
      marginTop: 10,
      flex: 'none'
    }
  }), c))), (irreversible || audit) && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '12px 0 0',
      fontSize: 13,
      color: 'var(--text-secondary)',
      borderTop: '1px solid var(--line)',
      paddingTop: 10
    }
  }, irreversible && 'This action cannot be reversed. ', audit && 'This action will be recorded.'));
}
Object.assign(__ds_scope, { ConsequencePanel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/command/ConsequencePanel.jsx", error: String((e && e.message) || e) }); }

// components/command/Directive.jsx
try { (() => {
const statusTone = {
  PENDING: 'var(--text-meta)',
  ACKNOWLEDGED: 'var(--authorized)',
  ACTIVE: 'var(--attention)',
  COMPLETED: 'var(--authorized)',
  REVOKED: 'var(--restricted)',
  FAILED: 'var(--controlled)'
};
function Directive({
  number,
  instruction,
  recipient,
  issuer,
  issued,
  deadline,
  priority = 'STANDARD',
  status = 'PENDING',
  onAcknowledge,
  onDecline,
  style
}) {
  return /*#__PURE__*/React.createElement("article", {
    style: {
      border: '1px solid var(--line-strong)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      padding: '24px 28px',
      ...style
    }
  }, /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'baseline',
      marginBottom: 18
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 15,
      fontWeight: 700,
      letterSpacing: '0.1em',
      textTransform: 'uppercase'
    }
  }, "DIRECTIVE ", number), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: statusTone[status] || 'var(--text-meta)'
    }
  }, status)), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '0 0 22px',
      fontSize: 19,
      lineHeight: 1.45,
      maxWidth: '36em'
    }
  }, instruction), /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: 0,
      display: 'flex',
      flexWrap: 'wrap',
      gap: '10px 28px',
      fontSize: 13,
      borderTop: '1px solid var(--line)',
      paddingTop: 14
    }
  }, [['RECIPIENT', recipient], ['ISSUED BY', issuer], ['ISSUED', issued], ['DEADLINE', deadline], ['PRIORITY', priority]].filter(([, v]) => v).map(([k, v]) => /*#__PURE__*/React.createElement("div", {
    key: k
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      marginBottom: 2
    }
  }, k), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, v)))), (onAcknowledge || onDecline) && /*#__PURE__*/React.createElement("footer", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      marginTop: 22
    }
  }, onDecline ? /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onDecline,
    style: btn(false)
  }, "Decline if unauthorized") : /*#__PURE__*/React.createElement("span", null), onAcknowledge && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onAcknowledge,
    style: btn(true)
  }, "Acknowledge")));
}
const btn = primary => ({
  fontFamily: 'var(--font-directive)',
  fontSize: 12,
  fontWeight: 600,
  letterSpacing: '0.07em',
  textTransform: 'uppercase',
  padding: '8px 18px',
  cursor: 'pointer',
  borderRadius: 'var(--radius-standard)',
  background: primary ? 'var(--surface-authority)' : 'transparent',
  color: primary ? 'var(--text-inverse)' : 'var(--text-secondary)',
  border: primary ? '1px solid var(--surface-authority)' : '1px solid var(--line-strong)'
});
Object.assign(__ds_scope, { Directive });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/command/Directive.jsx", error: String((e && e.message) || e) }); }

// components/display/Avatar.jsx
try { (() => {
function Avatar({
  name = '',
  size = 36,
  ring = 'none',
  src,
  style
}) {
  const initials = name.split(/\s+/).map(w => w[0]).slice(0, 2).join('').toUpperCase();
  const ringColor = {
    none: null,
    selected: 'var(--auburn)',
    watch: 'var(--observation-active)',
    escalated: 'var(--observation-escalated)'
  }[ring];
  const pad = ringColor ? Math.max(8, size * 0.28) : 0;
  const core = /*#__PURE__*/React.createElement("span", {
    style: {
      width: size,
      height: size,
      borderRadius: '50%',
      background: 'var(--bureau)',
      color: 'var(--ink-soft)',
      border: '1px solid var(--line-strong)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-directive)',
      fontWeight: 600,
      fontSize: size * 0.36,
      letterSpacing: '0.04em',
      overflow: 'hidden',
      flex: 'none'
    }
  }, src ? /*#__PURE__*/React.createElement("img", {
    src: src,
    alt: name,
    style: {
      width: '100%',
      height: '100%',
      objectFit: 'cover'
    }
  }) : initials);
  if (!ringColor) return /*#__PURE__*/React.createElement("span", {
    title: name,
    style: style
  }, core);
  const d = size + pad * 2;
  return /*#__PURE__*/React.createElement("span", {
    title: name,
    style: {
      position: 'relative',
      display: 'inline-flex',
      width: d,
      height: d,
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement("svg", {
    "aria-hidden": "true",
    width: d,
    height: d,
    viewBox: `0 0 ${d} ${d}`,
    style: {
      position: 'absolute',
      inset: 0
    }
  }, /*#__PURE__*/React.createElement("circle", {
    cx: d / 2,
    cy: d / 2,
    r: size / 2 + pad * 0.45,
    fill: "none",
    stroke: ringColor,
    strokeOpacity: "0.55",
    strokeWidth: "1"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: d / 2,
    cy: d / 2,
    r: size / 2 + pad * 0.9,
    fill: "none",
    stroke: ringColor,
    strokeOpacity: "0.22",
    strokeWidth: "1"
  })), core);
}
Object.assign(__ds_scope, { Avatar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Avatar.jsx", error: String((e && e.message) || e) }); }

// components/display/Badge.jsx
try { (() => {
const tones = {
  neutral: {
    color: 'var(--text-secondary)',
    border: 'var(--line-strong)',
    dot: 'var(--neutral)'
  },
  authorized: {
    color: 'var(--authorized)',
    border: 'var(--authorized)',
    dot: 'var(--authorized)'
  },
  attention: {
    color: 'var(--attention)',
    border: 'var(--attention)',
    dot: 'var(--attention)'
  },
  restricted: {
    color: 'var(--restricted)',
    border: 'var(--restricted)',
    dot: 'var(--restricted)'
  },
  controlled: {
    color: 'var(--controlled)',
    border: 'var(--controlled)',
    dot: 'var(--controlled)'
  },
  ink: {
    color: 'var(--text-inverse)',
    border: 'var(--ink)',
    dot: 'transparent',
    background: 'var(--ink)'
  }
};
function Badge({
  tone = 'neutral',
  dot = true,
  children,
  style
}) {
  const t = tones[tone] || tones.neutral;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      padding: '2px 8px',
      border: `1px solid ${t.border}`,
      borderRadius: 'var(--radius-standard)',
      background: t.background || 'transparent',
      color: t.color,
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      whiteSpace: 'nowrap',
      ...style
    }
  }, dot && t.dot !== 'transparent' && /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 6,
      height: 6,
      borderRadius: '50%',
      background: t.dot
    }
  }), children);
}
Object.assign(__ds_scope, { Badge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Badge.jsx", error: String((e && e.message) || e) }); }

// components/display/DataTable.jsx
try { (() => {
function DataTable({
  columns = [],
  rows = [],
  density = 'comfortable',
  selectedIndex = -1,
  onRowClick,
  style
}) {
  const pad = density === 'compact' ? '6px 12px' : '10px 12px';
  return /*#__PURE__*/React.createElement("table", {
    style: {
      width: '100%',
      borderCollapse: 'collapse',
      fontSize: 13,
      fontVariantNumeric: 'tabular-nums',
      ...style
    }
  }, /*#__PURE__*/React.createElement("thead", null, /*#__PURE__*/React.createElement("tr", null, columns.map((c, i) => /*#__PURE__*/React.createElement("th", {
    key: i,
    scope: "col",
    style: {
      textAlign: c.align || 'left',
      padding: pad,
      borderBottom: '1px solid var(--line-strong)',
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      color: 'var(--text-meta)',
      whiteSpace: 'nowrap'
    }
  }, c.header)))), /*#__PURE__*/React.createElement("tbody", null, rows.map((row, ri) => /*#__PURE__*/React.createElement("tr", {
    key: ri,
    onClick: onRowClick ? () => onRowClick(row, ri) : undefined,
    style: {
      cursor: onRowClick ? 'pointer' : 'default',
      background: ri === selectedIndex ? 'var(--surface-inset)' : ri % 2 ? 'color-mix(in srgb, var(--surface-inset) 30%, transparent)' : 'transparent',
      boxShadow: ri === selectedIndex ? 'inset 2px 0 0 var(--auburn)' : 'none'
    }
  }, columns.map((c, ci) => /*#__PURE__*/React.createElement("td", {
    key: ci,
    style: {
      padding: pad,
      borderBottom: '1px solid var(--line)',
      textAlign: c.align || 'left',
      color: 'var(--text-primary)'
    }
  }, c.render ? c.render(row) : row[c.key]))))));
}
Object.assign(__ds_scope, { DataTable });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/DataTable.jsx", error: String((e && e.message) || e) }); }

// components/display/KeyValue.jsx
try { (() => {
function KeyValue({
  items = [],
  columns = 1,
  size = 'md',
  style
}) {
  return /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: 0,
      display: 'grid',
      gridTemplateColumns: `repeat(${columns}, 1fr)`,
      gap: size === 'sm' ? '10px 20px' : '16px 24px',
      ...style
    }
  }, items.map(([k, v], i) => /*#__PURE__*/React.createElement("div", {
    key: i
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      color: 'var(--text-meta)',
      marginBottom: 3
    }
  }, k), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0,
      fontSize: size === 'sm' ? 13 : 15,
      color: 'var(--text-primary)'
    }
  }, v))));
}
Object.assign(__ds_scope, { KeyValue });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/KeyValue.jsx", error: String((e && e.message) || e) }); }

// components/display/Panel.jsx
try { (() => {
function Panel({
  kicker,
  title,
  actions,
  children,
  footer,
  style
}) {
  return /*#__PURE__*/React.createElement("section", {
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--radius-standard)',
      boxShadow: 'var(--shadow-card)',
      ...style
    }
  }, (kicker || title || actions) && /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 12,
      padding: '12px 16px',
      borderBottom: '1px solid var(--line)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, kicker && /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      color: 'var(--text-meta)'
    }
  }, kicker), title && /*#__PURE__*/React.createElement("h2", {
    style: {
      margin: 0,
      fontSize: 16,
      fontWeight: 600
    }
  }, title)), actions), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: 16
    }
  }, children), footer && /*#__PURE__*/React.createElement("footer", {
    style: {
      padding: '10px 16px',
      borderTop: '1px solid var(--line)',
      fontSize: 12,
      color: 'var(--text-meta)'
    }
  }, footer));
}
Object.assign(__ds_scope, { Panel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Panel.jsx", error: String((e && e.message) || e) }); }

// components/display/Stat.jsx
try { (() => {
function Stat({
  label,
  value,
  detail,
  tone = 'neutral',
  style
}) {
  const color = {
    neutral: 'var(--text-primary)',
    attention: 'var(--attention)',
    controlled: 'var(--controlled)',
    authorized: 'var(--authorized)'
  }[tone];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      color: 'var(--text-meta)',
      marginBottom: 4
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 26,
      fontWeight: 600,
      lineHeight: 1.1,
      color,
      fontVariantNumeric: 'tabular-nums'
    }
  }, value), detail && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 12,
      color: 'var(--text-meta)',
      marginTop: 4
    }
  }, detail));
}
Object.assign(__ds_scope, { Stat });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Stat.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Dialog.jsx
try { (() => {
function Dialog({
  open = true,
  kicker,
  title,
  children,
  actions,
  onClose,
  width = 440,
  style
}) {
  if (!open) return null;
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'fixed',
      inset: 0,
      background: 'rgba(27,24,18,0.35)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 100
    }
  }, /*#__PURE__*/React.createElement("div", {
    role: "dialog",
    "aria-modal": "true",
    "aria-label": title,
    onClick: e => e.stopPropagation(),
    style: {
      width,
      maxWidth: '90vw',
      background: 'var(--surface-card)',
      border: '1px solid var(--line-strong)',
      borderRadius: 'var(--radius-standard)',
      boxShadow: 'var(--shadow-overlay)',
      padding: '28px 32px',
      ...style
    }
  }, kicker && /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      color: 'var(--text-meta)',
      marginBottom: 10
    }
  }, kicker), title && /*#__PURE__*/React.createElement("h2", {
    style: {
      margin: '0 0 14px',
      fontSize: 20,
      fontWeight: 600
    }
  }, title), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 14,
      lineHeight: 1.55
    }
  }, children), actions && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      gap: 12,
      marginTop: 28
    }
  }, actions)));
}
Object.assign(__ds_scope, { Dialog });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Dialog.jsx", error: String((e && e.message) || e) }); }

// components/feedback/EmptyState.jsx
try { (() => {
function EmptyState({
  label = 'NO RECORDS',
  children,
  action,
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      textAlign: 'center',
      padding: '48px 24px',
      color: 'var(--text-meta)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    "aria-hidden": "true",
    style: {
      width: 40,
      margin: '0 auto 14px',
      borderTop: '1px solid var(--line-strong)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 12,
      fontWeight: 600,
      letterSpacing: '0.08em'
    }
  }, label), children && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '8px 0 0',
      fontSize: 13
    }
  }, children), action && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 16
    }
  }, action));
}
Object.assign(__ds_scope, { EmptyState });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/EmptyState.jsx", error: String((e && e.message) || e) }); }

// components/feedback/Notice.jsx
try { (() => {
const tones = {
  neutral: {
    rule: 'var(--line-strong)',
    label: 'NOTICE'
  },
  authorized: {
    rule: 'var(--authorized)',
    label: 'APPROVED'
  },
  attention: {
    rule: 'var(--attention)',
    label: 'ATTENTION'
  },
  restricted: {
    rule: 'var(--restricted)',
    label: 'RESTRICTED'
  },
  controlled: {
    rule: 'var(--controlled)',
    label: 'CONTROL'
  }
};
function Notice({
  tone = 'neutral',
  label,
  children,
  actions,
  style
}) {
  const t = tones[tone] || tones.neutral;
  return /*#__PURE__*/React.createElement("div", {
    role: tone === 'controlled' ? 'alert' : 'status',
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--radius-standard)',
      padding: '12px 16px',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 14,
      borderTop: `2px solid ${t.rule}`,
      alignSelf: 'center',
      flex: 'none'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)'
    }
  }, label || t.label)), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 14,
      marginTop: 6,
      color: 'var(--text-primary)'
    }
  }, children), actions && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 10,
      marginTop: 10
    }
  }, actions));
}
Object.assign(__ds_scope, { Notice });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/Notice.jsx", error: String((e && e.message) || e) }); }

// components/forms/Checkbox.jsx
try { (() => {
function Checkbox({
  label,
  checked = false,
  onChange,
  disabled = false,
  style
}) {
  const id = React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: id,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 10,
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      fontSize: 14,
      ...style
    }
  }, /*#__PURE__*/React.createElement("input", {
    id: id,
    type: "checkbox",
    checked: checked,
    disabled: disabled,
    onChange: e => onChange && onChange(e.target.checked),
    style: {
      position: 'absolute',
      opacity: 0,
      width: 16,
      height: 16
    }
  }), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 15,
      height: 15,
      flex: 'none',
      border: '1px solid var(--line-strong)',
      borderRadius: 1,
      background: checked ? 'var(--ink)' : 'var(--surface-raised)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: 'var(--text-inverse)',
      fontSize: 11,
      lineHeight: 1,
      transition: 'background var(--motion-standard)'
    }
  }, checked ? '✓' : ''), label);
}
Object.assign(__ds_scope, { Checkbox });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Checkbox.jsx", error: String((e && e.message) || e) }); }

// components/forms/Radio.jsx
try { (() => {
function Radio({
  label,
  name,
  value,
  checked = false,
  onChange,
  disabled = false,
  style
}) {
  const id = React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: id,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 10,
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      fontSize: 14,
      ...style
    }
  }, /*#__PURE__*/React.createElement("input", {
    id: id,
    type: "radio",
    name: name,
    value: value,
    checked: checked,
    disabled: disabled,
    onChange: () => onChange && onChange(value),
    style: {
      position: 'absolute',
      opacity: 0,
      width: 16,
      height: 16
    }
  }), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 15,
      height: 15,
      flex: 'none',
      border: checked ? '1px solid var(--ink)' : '1px solid var(--line-strong)',
      borderRadius: '50%',
      background: 'var(--surface-raised)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      transition: 'border-color var(--motion-standard)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 7,
      height: 7,
      borderRadius: '50%',
      background: checked ? 'var(--ink)' : 'transparent',
      transition: 'background var(--motion-standard)'
    }
  })), label);
}
Object.assign(__ds_scope, { Radio });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Radio.jsx", error: String((e && e.message) || e) }); }

// components/forms/Switch.jsx
try { (() => {
function Switch({
  label,
  checked = false,
  onChange,
  disabled = false,
  style
}) {
  const id = React.useId();
  return /*#__PURE__*/React.createElement("label", {
    htmlFor: id,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 10,
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.5 : 1,
      fontSize: 14,
      ...style
    }
  }, /*#__PURE__*/React.createElement("input", {
    id: id,
    type: "checkbox",
    role: "switch",
    checked: checked,
    disabled: disabled,
    onChange: e => onChange && onChange(e.target.checked),
    style: {
      position: 'absolute',
      opacity: 0,
      width: 32,
      height: 18
    }
  }), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      width: 32,
      height: 18,
      flex: 'none',
      border: '1px solid var(--line-strong)',
      borderRadius: 2,
      position: 'relative',
      background: checked ? 'var(--ink)' : 'var(--surface-inset)',
      transition: 'background var(--motion-standard)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 2,
      left: checked ? 16 : 2,
      width: 12,
      height: 12,
      background: checked ? 'var(--paper)' : 'var(--graphite)',
      transition: 'left var(--motion-standard), background var(--motion-standard)'
    }
  })), label);
}
Object.assign(__ds_scope, { Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Switch.jsx", error: String((e && e.message) || e) }); }

// components/forms/TextField.jsx
try { (() => {
const fieldLabelStyle = {
  fontFamily: 'var(--font-directive)',
  fontSize: 12,
  fontWeight: 600,
  letterSpacing: '0.07em',
  textTransform: 'uppercase',
  color: 'var(--text-secondary)',
  display: 'block',
  marginBottom: 6
};
const fieldInputStyle = {
  fontFamily: 'var(--font-official)',
  fontSize: 14,
  color: 'var(--text-primary)',
  background: 'var(--surface-raised)',
  border: '1px solid var(--line-strong)',
  borderRadius: 'var(--radius-standard)',
  padding: '8px 10px',
  width: '100%',
  boxSizing: 'border-box',
  outline: 'none'
};
function TextField({
  label,
  value,
  onChange,
  placeholder,
  help,
  error,
  multiline = false,
  rows = 3,
  disabled = false,
  style
}) {
  const id = React.useId();
  const Tag = multiline ? 'textarea' : 'input';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      ...style
    }
  }, /*#__PURE__*/React.createElement("label", {
    htmlFor: id,
    style: fieldLabelStyle
  }, label), /*#__PURE__*/React.createElement(Tag, {
    id: id,
    value: value,
    placeholder: placeholder,
    disabled: disabled,
    rows: multiline ? rows : undefined,
    onChange: e => onChange && onChange(e.target.value),
    "aria-invalid": !!error,
    style: {
      ...fieldInputStyle,
      resize: multiline ? 'vertical' : undefined,
      borderColor: error ? 'var(--directive-red)' : 'var(--line-strong)',
      opacity: disabled ? 0.5 : 1
    }
  }), help && !error && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '5px 0 0',
      fontSize: 12,
      color: 'var(--text-meta)'
    }
  }, help), error && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '5px 0 0',
      fontSize: 12,
      color: 'var(--directive-red)'
    }
  }, error));
}
Object.assign(__ds_scope, { fieldLabelStyle, fieldInputStyle, TextField });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/TextField.jsx", error: String((e && e.message) || e) }); }

// components/forms/Select.jsx
try { (() => {
function Select({
  label,
  value,
  onChange,
  options = [],
  disabled = false,
  style
}) {
  const id = React.useId();
  return /*#__PURE__*/React.createElement("div", {
    style: {
      ...style
    }
  }, /*#__PURE__*/React.createElement("label", {
    htmlFor: id,
    style: __ds_scope.fieldLabelStyle
  }, label), /*#__PURE__*/React.createElement("select", {
    id: id,
    value: value,
    disabled: disabled,
    onChange: e => onChange && onChange(e.target.value),
    style: {
      ...__ds_scope.fieldInputStyle,
      appearance: 'none',
      opacity: disabled ? 0.5 : 1,
      backgroundImage: 'linear-gradient(45deg, transparent 50%, var(--graphite) 50%), linear-gradient(135deg, var(--graphite) 50%, transparent 50%)',
      backgroundPosition: 'calc(100% - 16px) 55%, calc(100% - 11px) 55%',
      backgroundSize: '5px 5px',
      backgroundRepeat: 'no-repeat'
    }
  }, options.map(o => typeof o === 'string' ? /*#__PURE__*/React.createElement("option", {
    key: o,
    value: o
  }, o) : /*#__PURE__*/React.createElement("option", {
    key: o.value,
    value: o.value
  }, o.label))));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Select.jsx", error: String((e && e.message) || e) }); }

// components/records/AuditRecord.jsx
try { (() => {
function AuditRecord({
  entries = [],
  style
}) {
  return /*#__PURE__*/React.createElement("table", {
    style: {
      width: '100%',
      borderCollapse: 'collapse',
      fontSize: 12,
      fontVariantNumeric: 'tabular-nums',
      ...style
    }
  }, /*#__PURE__*/React.createElement("thead", null, /*#__PURE__*/React.createElement("tr", null, ['TIME', 'ACTOR', 'AUTHORITY', 'ACTION', 'TARGET', 'RESULT'].map(h => /*#__PURE__*/React.createElement("th", {
    key: h,
    scope: "col",
    style: {
      textAlign: 'left',
      padding: '6px 10px',
      borderBottom: '1px solid var(--line-strong)',
      fontFamily: 'var(--font-directive)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)'
    }
  }, h)))), /*#__PURE__*/React.createElement("tbody", null, entries.map((e, i) => /*#__PURE__*/React.createElement("tr", {
    key: i
  }, /*#__PURE__*/React.createElement("td", {
    style: td
  }, e.time), /*#__PURE__*/React.createElement("td", {
    style: td
  }, e.actor), /*#__PURE__*/React.createElement("td", {
    style: td
  }, e.authority), /*#__PURE__*/React.createElement("td", {
    style: td
  }, e.action), /*#__PURE__*/React.createElement("td", {
    style: td
  }, e.target), /*#__PURE__*/React.createElement("td", {
    style: {
      ...td,
      color: e.result === 'DENIED' ? 'var(--restricted)' : 'var(--text-secondary)',
      fontFamily: 'var(--font-directive)',
      letterSpacing: '0.06em'
    }
  }, e.result)))));
}
const td = {
  padding: '6px 10px',
  borderBottom: '1px solid var(--line)',
  color: 'var(--text-primary)'
};
Object.assign(__ds_scope, { AuditRecord });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/records/AuditRecord.jsx", error: String((e && e.message) || e) }); }

// components/records/Contract.jsx
try { (() => {
const stateTone = {
  DRAFT: 'var(--contract-draft)',
  OFFERED: 'var(--attention)',
  ACTIVE: 'var(--contract-active)',
  FULFILLED: 'var(--contract-active)',
  BREACHED: 'var(--contract-breached)',
  REVOKED: 'var(--restricted)',
  TERMINATED: 'var(--contract-terminated)'
};
function Contract({
  number,
  status = 'DRAFT',
  parties = [],
  facts = [],
  onViewTerms,
  style
}) {
  return /*#__PURE__*/React.createElement("article", {
    style: {
      border: '1px solid var(--line-strong)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'baseline',
      padding: '14px 18px',
      borderBottom: '1px solid var(--line)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 14,
      fontWeight: 700,
      letterSpacing: '0.1em'
    }
  }, "CONTRACT ", number), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: stateTone[status] || 'var(--text-meta)'
    }
  }, status)), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px 18px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      marginBottom: 8
    }
  }, "PARTIES"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 2,
      marginBottom: 16,
      fontSize: 15
    }
  }, /*#__PURE__*/React.createElement("span", null, parties[0]), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      color: 'var(--text-meta)'
    }
  }, "\u2195"), /*#__PURE__*/React.createElement("span", null, parties[1])), /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: 0,
      display: 'grid',
      gridTemplateColumns: 'repeat(2, 1fr)',
      gap: '10px 18px',
      fontSize: 13,
      borderTop: '1px solid var(--line)',
      paddingTop: 12
    }
  }, facts.map(([k, v]) => /*#__PURE__*/React.createElement("div", {
    key: k
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      color: 'var(--text-meta)',
      textTransform: 'uppercase',
      marginBottom: 2
    }
  }, k), /*#__PURE__*/React.createElement("dd", {
    style: {
      margin: 0
    }
  }, v))))), onViewTerms && /*#__PURE__*/React.createElement("footer", {
    style: {
      padding: '10px 18px',
      borderTop: '1px solid var(--line)'
    }
  }, /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onViewTerms,
    style: {
      background: 'none',
      border: 'none',
      padding: 0,
      cursor: 'pointer',
      color: 'var(--text-link)',
      fontSize: 13,
      borderBottom: '1px solid currentColor'
    }
  }, "View terms")));
}
Object.assign(__ds_scope, { Contract });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/records/Contract.jsx", error: String((e && e.message) || e) }); }

// components/records/Observation.jsx
try { (() => {
function Observation({
  number,
  subject,
  time,
  behavior,
  source,
  confidence = 'REPORTED',
  escalated = false,
  style
}) {
  const ringColor = escalated ? 'var(--observation-escalated)' : 'var(--observation-passive)';
  return /*#__PURE__*/React.createElement("article", {
    style: {
      display: 'flex',
      gap: 14,
      padding: '12px 0',
      borderBottom: '1px solid var(--line)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    "aria-hidden": "true",
    style: {
      flex: 'none',
      marginTop: 3
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "18",
    height: "18",
    viewBox: "0 0 18 18"
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "9",
    cy: "9",
    r: "2.5",
    fill: ringColor
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "9",
    cy: "9",
    r: "6",
    fill: "none",
    stroke: ringColor,
    strokeOpacity: "0.5",
    strokeWidth: "1"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "9",
    cy: "9",
    r: "8.5",
    fill: "none",
    stroke: ringColor,
    strokeOpacity: "0.2",
    strokeWidth: "1"
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 14,
      fontSize: 11,
      fontFamily: 'var(--font-directive)',
      fontWeight: 600,
      letterSpacing: '0.07em',
      color: 'var(--text-meta)',
      marginBottom: 3
    }
  }, /*#__PURE__*/React.createElement("span", null, "OBS ", number), subject && /*#__PURE__*/React.createElement("span", null, "SUBJECT ", subject), /*#__PURE__*/React.createElement("span", null, time)), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: 0,
      fontSize: 14
    }
  }, behavior), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 12,
      color: 'var(--text-meta)',
      marginTop: 3
    }
  }, source, confidence ? ` · ${confidence}` : '')));
}
Object.assign(__ds_scope, { Observation });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/records/Observation.jsx", error: String((e && e.message) || e) }); }

// components/records/SubjectRecord.jsx
try { (() => {
function SubjectRecord({
  id,
  name,
  ring = 'selected',
  fields = [],
  note,
  style
}) {
  return /*#__PURE__*/React.createElement("article", {
    style: {
      border: '1px solid var(--line-strong)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--radius-standard)',
      padding: '20px 24px',
      ...style
    }
  }, /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      gap: 18,
      alignItems: 'center',
      marginBottom: 18
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Avatar, {
    name: name,
    size: 44,
    ring: ring
  }), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: '0.1em',
      color: 'var(--text-meta)'
    }
  }, "SUBJECT ", id), /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 20,
      fontWeight: 600
    }
  }, name))), /*#__PURE__*/React.createElement(__ds_scope.KeyValue, {
    items: fields,
    columns: 3,
    size: "sm"
  }), note && /*#__PURE__*/React.createElement("p", {
    style: {
      margin: '16px 0 0',
      fontFamily: 'var(--font-personal)',
      fontSize: 14,
      lineHeight: 1.6,
      color: 'var(--text-secondary)',
      borderTop: '1px solid var(--line)',
      paddingTop: 12
    }
  }, note));
}
Object.assign(__ds_scope, { SubjectRecord });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/records/SubjectRecord.jsx", error: String((e && e.message) || e) }); }

// ui_kits/bureau/App.jsx
try { (() => {
const DSA = window.Directive01DesignSystem_03c333;
const {
  AuthorityBadge: ABA,
  Avatar: AvA
} = DSA;
const NAV = [['office', 'Office'], ['personnel', 'Personnel'], ['subject', 'Subject 034'], ['contract', 'Contract'], ['directive', 'Directive'], ['agents', 'Agents'], ['control', 'Control']];
function BureauApp() {
  const [screen, setScreen] = React.useState(localStorage.getItem('d01_screen') || 'office');
  const [ack, setAck] = React.useState(false);
  const [revoked, setRevoked] = React.useState(false);
  const go = s => {
    const t = s === 'observation' ? 'subject' : s;
    setScreen(t);
    localStorage.setItem('d01_screen', t);
  };
  const body = {
    office: /*#__PURE__*/React.createElement(ScreenOffice, {
      go: go
    }),
    personnel: /*#__PURE__*/React.createElement(ScreenPersonnel, {
      go: go
    }),
    subject: /*#__PURE__*/React.createElement(ScreenSubject, {
      go: go
    }),
    contract: /*#__PURE__*/React.createElement(ScreenContract, {
      go: go
    }),
    directive: /*#__PURE__*/React.createElement(ScreenDirective, {
      go: go,
      acknowledged: ack,
      onAck: () => setAck(true)
    }),
    agents: /*#__PURE__*/React.createElement(ScreenAgents, null),
    control: /*#__PURE__*/React.createElement(ScreenControl, {
      go: go,
      revoked: revoked,
      onRevoke: () => setRevoked(true)
    })
  }[screen];
  const control = screen === 'control';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      minHeight: '100vh'
    }
  }, /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 20,
      padding: '10px 20px',
      borderBottom: '1px solid var(--line-strong)',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-directive)',
      fontWeight: 600,
      fontSize: 14,
      letterSpacing: '0.1em'
    }
  }, "directive", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--directive-red)'
    }
  }, "//"), "01"), /*#__PURE__*/React.createElement("span", {
    className: "d01-label"
  }, "Public Safety \xB7 Administration"), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement("input", {
    "aria-label": "Search records",
    placeholder: "Search records",
    style: {
      fontFamily: 'var(--font-official)',
      fontSize: 13,
      padding: '5px 10px',
      border: '1px solid var(--line)',
      background: 'var(--surface-raised)',
      width: 200
    }
  }), /*#__PURE__*/React.createElement(ABA, {
    level: "controlled",
    role: "You"
  }), /*#__PURE__*/React.createElement(AvA, {
    name: "M",
    size: 28,
    ring: control ? 'selected' : 'none'
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flex: 1
    }
  }, !control && /*#__PURE__*/React.createElement("nav", {
    "aria-label": "Primary",
    style: {
      width: 170,
      flex: 'none',
      borderRight: '1px solid var(--line)',
      padding: '16px 0',
      background: 'var(--surface-card)'
    }
  }, NAV.map(([id, label]) => /*#__PURE__*/React.createElement("button", {
    key: id,
    type: "button",
    onClick: () => go(id),
    style: {
      display: 'block',
      width: '100%',
      textAlign: 'left',
      padding: '8px 20px',
      background: screen === id ? 'var(--surface-inset)' : 'transparent',
      border: 'none',
      borderLeft: screen === id ? '2px solid var(--auburn)' : '2px solid transparent',
      cursor: 'pointer',
      fontFamily: 'var(--font-directive)',
      fontSize: 12,
      fontWeight: 600,
      letterSpacing: '0.07em',
      textTransform: 'uppercase',
      color: id === 'control' ? 'var(--wine)' : 'var(--text-secondary)'
    }
  }, label))), /*#__PURE__*/React.createElement("main", {
    style: {
      flex: 1,
      padding: control ? '24px 32px' : '20px 24px',
      background: 'var(--surface-page)',
      minWidth: 0
    }
  }, body)), /*#__PURE__*/React.createElement("footer", {
    style: {
      display: 'flex',
      gap: 24,
      padding: '6px 20px',
      borderTop: '1px solid var(--line)',
      background: 'var(--surface-card)',
      fontSize: 11,
      fontFamily: 'var(--font-directive)',
      letterSpacing: '0.06em',
      color: 'var(--text-meta)'
    }
  }, /*#__PURE__*/React.createElement("span", null, "SESSION 118-04"), /*#__PURE__*/React.createElement("span", null, "CLEARANCE LEVEL 04"), /*#__PURE__*/React.createElement("span", null, "AUDIT ACTIVE"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, "08:31")));
}
ReactDOM.createRoot(document.getElementById('root')).render(/*#__PURE__*/React.createElement(BureauApp, null));
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/bureau/App.jsx", error: String((e && e.message) || e) }); }

// ui_kits/bureau/Screens1.jsx
try { (() => {
const DS = window.Directive01DesignSystem_03c333;
const {
  Panel,
  Badge,
  Stat,
  DataTable,
  KeyValue,
  Button,
  Notice,
  Directive,
  ApprovalChain,
  Observation
} = DS;
function ScreenOffice({
  go
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '2fr 1fr',
      gap: 16,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: 'repeat(4,1fr)',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(Panel, null, /*#__PURE__*/React.createElement(Stat, {
    label: "Personnel on duty",
    value: "128",
    detail: "Divisions 1\u20134"
  })), /*#__PURE__*/React.createElement(Panel, null, /*#__PURE__*/React.createElement(Stat, {
    label: "Active operations",
    value: "7",
    detail: "2 in review"
  })), /*#__PURE__*/React.createElement(Panel, null, /*#__PURE__*/React.createElement(Stat, {
    label: "Pending directives",
    value: "4",
    tone: "attention",
    detail: "1 requires acknowledgement"
  })), /*#__PURE__*/React.createElement(Panel, null, /*#__PURE__*/React.createElement(Stat, {
    label: "Contracts active",
    value: "61",
    detail: "3 due for review"
  }))), /*#__PURE__*/React.createElement(Panel, {
    kicker: "Pending approvals",
    footer: "Updated 08:14"
  }, /*#__PURE__*/React.createElement(DataTable, {
    density: "compact",
    columns: [{
      header: 'Request',
      key: 'req'
    }, {
      header: 'From',
      key: 'from'
    }, {
      header: 'Authority',
      key: 'auth'
    }, {
      header: 'Status',
      render: r => /*#__PURE__*/React.createElement(Badge, {
        tone: r.tone
      }, r.status)
    }],
    rows: [{
      req: 'Access: observation record 034',
      from: 'K. Ito',
      auth: 'Level 03',
      status: 'Pending',
      tone: 'attention'
    }, {
      req: 'Assignment transfer — Division 2',
      from: 'S. Mori',
      auth: 'Level 02',
      status: 'Approved',
      tone: 'authorized'
    }, {
      req: 'Equipment requisition 118',
      from: 'A. Hayakawa',
      auth: 'Level 02',
      status: 'Pending',
      tone: 'attention'
    }],
    onRowClick: () => {}
  })), /*#__PURE__*/React.createElement(Panel, {
    kicker: "Recent incidents"
  }, /*#__PURE__*/React.createElement(DataTable, {
    density: "compact",
    columns: [{
      header: 'Incident',
      key: 'id'
    }, {
      header: 'Location',
      key: 'loc'
    }, {
      header: 'Division',
      key: 'div'
    }, {
      header: 'Status',
      render: r => /*#__PURE__*/React.createElement(Badge, {
        tone: r.tone
      }, r.status)
    }],
    rows: [{
      id: '22-B',
      loc: 'Kyoto station district',
      div: 'Division 4',
      status: 'Contained',
      tone: 'authorized'
    }, {
      id: '22-C',
      loc: 'Warehouse row 9',
      div: 'Division 2',
      status: 'Under review',
      tone: 'attention'
    }]
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(Directive, {
    number: "071",
    instruction: "Remain available pending further assignment.",
    recipient: "Division Four",
    issuer: "Director",
    issued: "07:42",
    deadline: "18:00",
    status: "PENDING",
    onAcknowledge: () => {},
    onDecline: () => {}
  }), /*#__PURE__*/React.createElement(Panel, {
    kicker: "Observation \u2014 today"
  }, /*#__PURE__*/React.createElement(Observation, {
    number: "81",
    subject: "034",
    time: "08:14",
    behavior: "Arrived at assigned location.",
    source: "Field report",
    confidence: "VERIFIED"
  }), /*#__PURE__*/React.createElement(Observation, {
    number: "83",
    subject: "051",
    time: "09:40",
    behavior: "Submitted weekly report.",
    source: "Records",
    confidence: "VERIFIED",
    style: {
      borderBottom: 'none'
    }
  }), /*#__PURE__*/React.createElement(Button, {
    variant: "ghost",
    size: "sm",
    style: {
      marginTop: 8
    },
    onClick: () => go('observation')
  }, "View all"))));
}
function ScreenPersonnel({
  go
}) {
  const rows = [{
    name: 'Makima',
    rank: 'Director-adjacent',
    div: 'Special Division 4',
    sup: '—',
    status: 'ACTIVE',
    tone: 'authorized',
    auth: '04'
  }, {
    name: 'Hayakawa, Aki',
    rank: 'Senior agent',
    div: 'Special Division 4',
    sup: 'Makima',
    status: 'ACTIVE',
    tone: 'authorized',
    auth: '02'
  }, {
    name: 'Denji',
    rank: 'Subject / agent',
    div: 'Special Division 4',
    sup: 'Makima',
    status: 'OBSERVED',
    tone: 'attention',
    auth: '01'
  }, {
    name: 'Power',
    rank: 'Fiend / agent',
    div: 'Special Division 4',
    sup: 'Hayakawa, A.',
    status: 'OBSERVED',
    tone: 'attention',
    auth: '01'
  }, {
    name: 'Kobeni, H.',
    rank: 'Agent',
    div: 'Special Division 4',
    sup: 'Hayakawa, A.',
    status: 'LEAVE',
    tone: 'neutral',
    auth: '01'
  }, {
    name: 'Mori, S.',
    rank: 'Division lead',
    div: 'Division 2',
    sup: 'Director',
    status: 'ACTIVE',
    tone: 'authorized',
    auth: '03'
  }];
  return /*#__PURE__*/React.createElement(Panel, {
    kicker: "Personnel directory",
    title: "Public Safety \u2014 all divisions",
    footer: "128 records \xB7 updated 08:00"
  }, /*#__PURE__*/React.createElement(DataTable, {
    columns: [{
      header: 'Name',
      key: 'name'
    }, {
      header: 'Rank',
      key: 'rank'
    }, {
      header: 'Division',
      key: 'div'
    }, {
      header: 'Supervisor',
      key: 'sup'
    }, {
      header: 'Authority',
      render: r => /*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-directive)',
          letterSpacing: '0.06em'
        }
      }, "LEVEL ", r.auth)
    }, {
      header: 'Status',
      render: r => /*#__PURE__*/React.createElement(Badge, {
        tone: r.tone
      }, r.status)
    }],
    rows: rows,
    onRowClick: r => {
      if (r.name === 'Denji') go('subject');
    }
  }));
}
Object.assign(window, {
  ScreenOffice,
  ScreenPersonnel
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/bureau/Screens1.jsx", error: String((e && e.message) || e) }); }

// ui_kits/bureau/Screens2.jsx
try { (() => {
const DS2 = window.Directive01DesignSystem_03c333;
const {
  Panel: P2,
  Badge: B2,
  KeyValue: KV2,
  Button: Btn2,
  SubjectRecord,
  Contract,
  Observation: Obs2,
  CommandChain,
  ApprovalChain: AC2,
  ClearanceGate,
  Notice: N2
} = DS2;
function ScreenSubject({
  go
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1.5fr 1fr',
      gap: 16,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(SubjectRecord, {
    id: "034",
    name: "Denji",
    ring: "watch",
    fields: [["Status", "Active"], ["Assignment", "Special Division 4"], ["Supervisor", "Makima"], ["Contract", "Valid"], ["Observation", "Ongoing"], ["Handler", "M."]],
    note: "He works better when he has something to look forward to. Keep the arrangement simple."
  }), /*#__PURE__*/React.createElement(P2, {
    kicker: "Observation history",
    footer: "Retention: 90 days \xB7 Review authority: Division Administration"
  }, /*#__PURE__*/React.createElement(N2, {
    tone: "attention",
    label: "ACTIVE WATCH",
    style: {
      marginBottom: 10
    }
  }, "This record is under authorized review. Reason: contract compliance. Authority: Division Administration. Review date: 1997.10.01."), /*#__PURE__*/React.createElement(Obs2, {
    number: "81",
    time: "08:14",
    behavior: "Arrived at assigned location.",
    source: "Field report",
    confidence: "VERIFIED"
  }), /*#__PURE__*/React.createElement(Obs2, {
    number: "82",
    time: "11:02",
    behavior: "Left assigned area without authorization.",
    source: "Escort report",
    confidence: "REPORTED",
    escalated: true
  }), /*#__PURE__*/React.createElement(Obs2, {
    number: "84",
    time: "12:30",
    behavior: "Returned. Cited food purchase.",
    source: "Self-report",
    confidence: "REPORTED",
    style: {
      borderBottom: 'none'
    }
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(Contract, {
    number: "04-118",
    status: "ACTIVE",
    parties: ["Public Safety", "Subject 034"],
    facts: [["Authorized actions", "3"], ["Restrictions", "2"], ["Effective", "1997.09.14"], ["Review", "1997.10.01"]],
    onViewTerms: () => go('contract')
  }), /*#__PURE__*/React.createElement(P2, {
    kicker: "Command chain"
  }, /*#__PURE__*/React.createElement(CommandChain, {
    selected: 3,
    links: [{
      name: 'Director',
      grant: 'full authority'
    }, {
      name: 'Makima',
      detail: 'Special Division 4',
      grant: 'operational authority'
    }, {
      name: 'Field escort',
      grant: 'assigned authority'
    }, {
      name: 'Subject 034'
    }]
  }))));
}
function ScreenContract({
  go
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 640,
      margin: '0 auto',
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(P2, {
    kicker: "Contract 04-118",
    title: "Terms of engagement"
  }, /*#__PURE__*/React.createElement(KV2, {
    columns: 2,
    items: [["Parties", "Public Safety ↕ Subject 034"], ["Duration", "Until review · 1997.10.01"]]
  }), /*#__PURE__*/React.createElement("hr", {
    className: "d01-rule",
    style: {
      margin: '16px 0'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: 20,
      fontSize: 14
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    className: "d01-label",
    style: {
      marginBottom: 6
    }
  }, "Obligations"), /*#__PURE__*/React.createElement("ul", {
    style: {
      margin: 0,
      paddingLeft: 18,
      lineHeight: 1.7
    }
  }, /*#__PURE__*/React.createElement("li", null, "Remain within assigned area."), /*#__PURE__*/React.createElement("li", null, "Report daily by 18:00."), /*#__PURE__*/React.createElement("li", null, "Accept assigned escort."))), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    className: "d01-label",
    style: {
      marginBottom: 6
    }
  }, "Benefits"), /*#__PURE__*/React.createElement("ul", {
    style: {
      margin: 0,
      paddingLeft: 18,
      lineHeight: 1.7
    }
  }, /*#__PURE__*/React.createElement("li", null, "Housing provided."), /*#__PURE__*/React.createElement("li", null, "Meals provided."), /*#__PURE__*/React.createElement("li", null, "Standard agent compensation.")))), /*#__PURE__*/React.createElement("hr", {
    className: "d01-rule",
    style: {
      margin: '16px 0'
    }
  }), /*#__PURE__*/React.createElement("div", {
    className: "d01-label",
    style: {
      marginBottom: 6
    }
  }, "Consequences of breach"), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: 0,
      fontSize: 14
    }
  }, "Escort privileges are suspended and the contract enters review. Termination requires Level 04 authority and is recorded.")), /*#__PURE__*/React.createElement(N2, {
    tone: "neutral",
    label: "CONSENT"
  }, "Both parties may request review at any time. Revocation terms are stated above; nothing in this agreement is hidden."), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between'
    }
  }, /*#__PURE__*/React.createElement(Btn2, {
    variant: "ghost",
    onClick: () => go('subject')
  }, "Return to record"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(Btn2, {
    variant: "secondary"
  }, "Request review"), /*#__PURE__*/React.createElement(Btn2, {
    variant: "primary"
  }, "Confirm the contract"))));
}
function ScreenDirective({
  go,
  acknowledged,
  onAck
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 560,
      margin: '48px auto 0',
      textAlign: 'center'
    }
  }, /*#__PURE__*/React.createElement("div", {
    className: "d01-label",
    style: {
      fontSize: 13,
      letterSpacing: '0.14em'
    }
  }, "DIRECTIVE 071"), /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 30,
      fontWeight: 600,
      margin: '28px 0 40px',
      lineHeight: 1.3
    }
  }, "Remain available."), /*#__PURE__*/React.createElement(KV2, {
    columns: 3,
    items: [["Report by", "18:00"], ["Authority", "Director"], ["Acknowledgement", "Required"]],
    style: {
      textAlign: 'left',
      maxWidth: 420,
      margin: '0 auto'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      marginTop: 56,
      maxWidth: 460,
      marginLeft: 'auto',
      marginRight: 'auto'
    }
  }, /*#__PURE__*/React.createElement(Btn2, {
    variant: "ghost"
  }, "Decline if unauthorized"), /*#__PURE__*/React.createElement(Btn2, {
    variant: "primary",
    onClick: onAck,
    disabled: acknowledged
  }, acknowledged ? 'Acknowledged' : 'Acknowledge')), acknowledged && /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 12,
      color: 'var(--text-meta)',
      marginTop: 20
    }
  }, "Acknowledged at 08:31. This acknowledgement has been recorded."));
}
Object.assign(window, {
  ScreenSubject,
  ScreenContract,
  ScreenDirective
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/bureau/Screens2.jsx", error: String((e && e.message) || e) }); }

// ui_kits/bureau/Screens3.jsx
try { (() => {
const DS3 = window.Directive01DesignSystem_03c333;
const {
  Panel: P3,
  Badge: B3,
  KeyValue: KV3,
  Button: Btn3,
  ConsequencePanel,
  AuditRecord,
  Handler,
  Notice: N3,
  AuthorityBadge: AB3
} = DS3;
function ScreenControl({
  go,
  revoked,
  onRevoke
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 520,
      margin: '40px auto 0'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'baseline',
      marginBottom: 28
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "d01-label",
    style: {
      color: 'var(--directive-red)',
      letterSpacing: '0.14em'
    }
  }, "CONTROL STATE"), /*#__PURE__*/React.createElement(AB3, {
    level: "controlled",
    role: "Session authority"
  })), !revoked ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 26,
      fontWeight: 600,
      margin: '0 0 28px'
    }
  }, "Revoke all access"), /*#__PURE__*/React.createElement(KV3, {
    columns: 2,
    items: [["Action", "Revoke credentials"], ["Authority", "Level 04"], ["Target", "Account 118-04"], ["Audit", "This action will be recorded."]]
  }), /*#__PURE__*/React.createElement(ConsequencePanel, {
    style: {
      margin: '24px 0'
    },
    irreversible: true,
    consequences: ["3 active sessions will terminate.", "2 dependent processes will stop.", "Access is revoked immediately."]
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between'
    }
  }, /*#__PURE__*/React.createElement(Btn3, {
    variant: "ghost",
    onClick: () => go('office')
  }, "Cancel"), /*#__PURE__*/React.createElement(Btn3, {
    variant: "control",
    onClick: onRevoke
  }, "Revoke"))) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 26,
      fontWeight: 600,
      margin: '0 0 16px'
    }
  }, "Access revoked."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontSize: 14,
      color: 'var(--text-secondary)',
      margin: '0 0 24px'
    }
  }, "Account 118-04 no longer holds credentials. The action was recorded at 08:32."), /*#__PURE__*/React.createElement(AuditRecord, {
    entries: [{
      time: '08:32',
      actor: 'You',
      authority: 'Level 04',
      action: 'Revoke credentials',
      target: 'Account 118-04',
      result: 'RECORDED'
    }]
  }), /*#__PURE__*/React.createElement(Btn3, {
    variant: "ghost",
    style: {
      marginTop: 24
    },
    onClick: () => go('office')
  }, "Return")));
}
function ScreenAgents() {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: 16,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 14
    }
  }, /*#__PURE__*/React.createElement(Handler, {
    agent: "Agent 07",
    handler: "Hazel",
    boundary: "Repository read \xB7 Build \xB7 Test",
    directive: "Review pull request 412",
    status: "ACTIVE",
    restricted: ['Merge', 'Deploy']
  }), /*#__PURE__*/React.createElement(Handler, {
    agent: "Agent 08",
    handler: "Operations",
    boundary: "Read-only",
    directive: "Analyze incident 22-B",
    status: "WORKING",
    restricted: ['Write access']
  }), /*#__PURE__*/React.createElement(Handler, {
    agent: "Agent 11",
    handler: "Records",
    boundary: "Archive read",
    directive: "Index 1996 case files",
    status: "IDLE"
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 14
    }
  }, /*#__PURE__*/React.createElement(P3, {
    kicker: "Agent contract \u2014 session",
    footer: "Contract expires when the session ends."
  }, /*#__PURE__*/React.createElement(KV3, {
    columns: 2,
    size: "sm",
    items: [["Agent", "07"], ["Handler", "Hazel"], ["Scope", "Session only"], ["Storage", "None"], ["Authorized tools", "Repository read, Build, Test"], ["Restricted", "Merge, Deploy"]]
  })), /*#__PURE__*/React.createElement(N3, {
    tone: "neutral",
    label: "PRINCIPLE"
  }, "Every autonomous agent has a named handler, an explicit boundary, and a contract. Nothing acts without provenance."), /*#__PURE__*/React.createElement(P3, {
    kicker: "Agent audit"
  }, /*#__PURE__*/React.createElement(AuditRecord, {
    entries: [{
      time: '08:12',
      actor: 'Agent 07',
      authority: 'Delegated',
      action: 'Read repository',
      target: 'core/service',
      result: 'RECORDED'
    }, {
      time: '08:15',
      actor: 'Agent 07',
      authority: 'Delegated',
      action: 'Run tests',
      target: 'suite 4',
      result: 'RECORDED'
    }, {
      time: '08:18',
      actor: 'Agent 07',
      authority: '—',
      action: 'Merge branch',
      target: 'main',
      result: 'DENIED'
    }]
  }))));
}
Object.assign(window, {
  ScreenControl,
  ScreenAgents
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/bureau/Screens3.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Button = __ds_scope.Button;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.AuthorityBadge = __ds_scope.AuthorityBadge;

__ds_ns.ClearanceGate = __ds_scope.ClearanceGate;

__ds_ns.FocusRing = __ds_scope.FocusRing;

__ds_ns.Handler = __ds_scope.Handler;

__ds_ns.ApprovalChain = __ds_scope.ApprovalChain;

__ds_ns.CommandChain = __ds_scope.CommandChain;

__ds_ns.ConsequencePanel = __ds_scope.ConsequencePanel;

__ds_ns.Directive = __ds_scope.Directive;

__ds_ns.Avatar = __ds_scope.Avatar;

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.DataTable = __ds_scope.DataTable;

__ds_ns.KeyValue = __ds_scope.KeyValue;

__ds_ns.Panel = __ds_scope.Panel;

__ds_ns.Stat = __ds_scope.Stat;

__ds_ns.Dialog = __ds_scope.Dialog;

__ds_ns.EmptyState = __ds_scope.EmptyState;

__ds_ns.Notice = __ds_scope.Notice;

__ds_ns.Checkbox = __ds_scope.Checkbox;

__ds_ns.Radio = __ds_scope.Radio;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.TextField = __ds_scope.TextField;

__ds_ns.AuditRecord = __ds_scope.AuditRecord;

__ds_ns.Contract = __ds_scope.Contract;

__ds_ns.Observation = __ds_scope.Observation;

__ds_ns.SubjectRecord = __ds_scope.SubjectRecord;

})();
