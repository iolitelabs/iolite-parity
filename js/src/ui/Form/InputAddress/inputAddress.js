// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import React, { Component, PropTypes } from 'react';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';

import util from '~/api/util';
import { nodeOrStringProptype } from '~/util/proptypes';

import IdentityIcon from '../../IdentityIcon';
import Input from '../Input';

import styles from './inputAddress.css';

class InputAddress extends Component {
  static propTypes = {
    accountsInfo: PropTypes.object,
    allowCopy: PropTypes.bool,
    className: PropTypes.string,
    disabled: PropTypes.bool,
    error: PropTypes.string,
    hideUnderline: PropTypes.bool,
    hint: nodeOrStringProptype(),
    label: nodeOrStringProptype(),
    onChange: PropTypes.func,
    onSubmit: PropTypes.func,
    small: PropTypes.bool,
    text: PropTypes.bool,
    tokens: PropTypes.object,
    value: PropTypes.string
  };

  static defaultProps = {
    allowCopy: true,
    hideUnderline: false,
    small: false
  };

  render () {
    const { className, disabled, error, hint, label, text, value } = this.props;
    const { accountsInfo, allowCopy, hideUnderline, onSubmit, small, tokens } = this.props;

    const account = accountsInfo[value] || tokens[value];

    const icon = this.renderIcon();

    const classes = [ className ];
    classes.push(!icon ? styles.inputEmpty : styles.input);

    const containerClasses = [ styles.container ];

    if (small) {
      containerClasses.push(styles.small);
    }

    return (
      <div className={ containerClasses.join(' ') }>
        <Input
          allowCopy={ allowCopy && (disabled ? value : false) }
          className={ classes.join(' ') }
          disabled={ disabled }
          error={ error }
          hideUnderline={ hideUnderline }
          hint={ hint }
          label={ label }
          onChange={ this.handleInputChange }
          onSubmit={ onSubmit }
          value={
            text && account
              ? account.name
              : value
          } />
        { icon }
      </div>
    );
  }

  renderIcon () {
    const { value, disabled, label, allowCopy, hideUnderline } = this.props;

    if (!value || !value.length || !util.isAddressValid(value)) {
      return null;
    }

    const classes = [disabled ? styles.iconDisabled : styles.icon];

    if (!label) {
      classes.push(styles.noLabel);
    }

    if (!allowCopy) {
      classes.push(styles.noCopy);
    }

    if (hideUnderline) {
      classes.push(styles.noUnderline);
    }

    return (
      <div className={ classes.join(' ') }>
        <IdentityIcon
          inline center
          address={ value } />
      </div>
    );
  }

  handleInputChange = (event, value) => {
    const isEmpty = (value.length === 0);

    this.setState({ isEmpty });

    if (!/^0x/.test(value) && util.isAddressValid(`0x${value}`)) {
      return this.props.onChange(event, `0x${value}`);
    }

    this.props.onChange(event, value);
  }
}

function mapStateToProps (state) {
  const { tokens } = state.balances;
  const { accountsInfo } = state.personal;

  return {
    accountsInfo,
    tokens
  };
}

function mapDispatchToProps (dispatch) {
  return bindActionCreators({}, dispatch);
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(InputAddress);
